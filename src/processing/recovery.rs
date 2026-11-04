//! Preserve CRC evidence before one fresh, non-resuming acquisition.
use super::Release;
use crate::{Args, db::Database};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

fn park(db: &Database, args: &Args, input: &Path, release: &Release) -> Result<()> {
    let source = &release.source;
    for ancestor in source.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        ensure!(
            !fs::symlink_metadata(ancestor)?.file_type().is_symlink(),
            "Cannot park a symlink payload"
        );
    }
    let source = source.canonicalize()?;
    ensure!(
        source.is_file() && source.starts_with(input.join(&args.user)),
        "CRC payload is outside user staging"
    );
    let directory = source
        .parent()
        .context("Missing payload parent")?
        .join(format!(".parked-no-burn-{}", rand::random::<u128>()));
    fs::create_dir(&directory)?;
    let target = directory.join(source.file_name().context("Missing payload filename")?);
    // Persist protection before moving bytes. An interruption requires review, never cleanup.
    db.query(
        "UPDATE releases SET parked=1 WHERE user=? AND item_key=?",
        &[&args.user, &release.key],
    )?;
    db.query(
        "INSERT INTO parked_payloads(path,user,item_key,parked_date,reason) VALUES(?,?,?,?,?)",
        &[
            &target.to_string_lossy(),
            &args.user,
            &release.key,
            &chrono::Utc::now().to_rfc3339(),
            "ZIP CRC failure; parked/no-burn",
        ],
    )?;
    fs::hard_link(&source, &target).context("Preserve CRC failed payload without overwriting")?;
    fs::File::open(&directory)?.sync_all()?;
    fs::remove_file(&source)?;
    fs::File::open(source.parent().unwrap())?.sync_all()?;
    eprintln!("  Parked/no-burn {}", target.display());
    Ok(())
}

pub(super) fn retry<T>(
    db: &Database,
    args: &Args,
    input: &Path,
    release: &Release,
    process: impl FnOnce(&Release) -> Result<T>,
) -> Result<T> {
    retry_with(
        db,
        args,
        input,
        release,
        |row| {
            let mut fresh_args = args.clone();
            fresh_args.force = false;
            if let Some(format) = &row[3] {
                fresh_args.format = <crate::Format as clap::ValueEnum>::from_str(format, true)
                    .map_err(anyhow::Error::msg)?;
            }
            let network = crate::net::Network::new(&args.cookies)?;
            let captured = crate::Release {
                key: release.key.clone(),
                artist: release.artist.clone(),
                title: row[1].clone().context("Missing recovery title")?,
                url: row[0].clone().context("Missing recovery download URL")?,
                publish: None,
                // Metadata is already captured. Private/deleted public pages must not block recovery.
                release_url: None,
                release_custom_host: None,
                private: false,
            };
            // A fresh HTTP stream and create-new publication, never resume or overwrite evidence.
            crate::download::fetch(&network, &fresh_args, &captured, db)
        },
        process,
    )
}

fn retry_with<T>(
    db: &Database,
    args: &Args,
    input: &Path,
    release: &Release,
    acquire: impl FnOnce(&crate::db::Row) -> Result<crate::download::Downloaded>,
    process: impl FnOnce(&Release) -> Result<T>,
) -> Result<T> {
    park(db, args, input, release)?;
    ensure!(
        !args.offline,
        "CRC payload parked/no-burn; offline mode prohibits re-acquisition"
    );
    let rows = db.query("SELECT download_url,release_title,crc_reacquired,format FROM releases WHERE user=? AND item_key=?", &[&args.user, &release.key])?;
    let row = rows.first().context("Missing CRC recovery history")?;
    ensure!(
        row[2].as_deref() == Some("0"),
        "CRC retry exhausted; release parked/no-burn for review"
    );
    // Consume the retry before any network I/O so interruption cannot cause a retry loop.
    db.query(
        "UPDATE releases SET crc_reacquired=1 WHERE user=? AND item_key=?",
        &[&args.user, &release.key],
    )?;
    crate::shutdown::check()?;
    eprintln!(
        "  CRC recovery: fully re-acquiring {} - {}",
        release.artist, release.album
    );
    let downloaded = acquire(row)?;
    let replacement = Release {
        source: downloaded.path,
        ..release.clone()
    };
    db.query("UPDATE releases SET file_path=?,format=?,download_date=?,last_error=NULL WHERE user=? AND item_key=?", &[&replacement.source.to_string_lossy(), &downloaded.format, &chrono::Utc::now().to_rfc3339(), &args.user, &release.key])?;
    match process(&replacement) {
        Err(error) if error.is::<crate::archive::CrcFailure>() => {
            park(db, args, input, &replacement)?;
            Err(error.context("Fresh acquisition also failed CRC; parked/no-burn for review"))
        }
        Err(error) if crate::shutdown::requested() => Err(error),
        result => {
            db.query(
                "UPDATE releases SET parked=0 WHERE user=? AND item_key=?",
                &[&args.user, &release.key],
            )?;
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn fixture(root: &Path) -> (Args, Database, Release) {
        let source = root.join("johnsmith/Artist/album.zip");
        fs::create_dir_all(source.parent().unwrap()).unwrap();
        fs::write(&source, b"bad archive evidence").unwrap();
        let args = Args::try_parse_from([
            "bcfunl",
            "--process",
            "--user",
            "johnsmith",
            "--output",
            root.to_str().unwrap(),
            "--serverdir",
            "/unused",
        ])
        .unwrap();
        let db = Database::open(root.join("collection.sqlite3").to_str().unwrap()).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path) VALUES('johnsmith','album1','Artist','Album','https://bandcamp.com/download','complete',?)", &[source.to_str().unwrap()]).unwrap();
        let release = Release {
            key: "album1".into(),
            artist: "Artist".into(),
            album: "Album".into(),
            source,
            genre: String::new(),
            country: String::new(),
            catalog: String::new(),
            publish: String::new(),
            url: String::new(),
        };
        (args, db, release)
    }

    fn fresh(release: &Release) -> Result<crate::download::Downloaded> {
        ensure!(
            !release.source.exists(),
            "Failed archive was not parked before acquisition"
        );
        fs::write(&release.source, b"fresh bytes")?;
        Ok(crate::download::Downloaded {
            path: release.source.clone(),
            format: "flac".into(),
            publish: None,
        })
    }

    #[test]
    fn fresh_retry_preserves_evidence_and_cleanup_protects_no_burn() {
        let root = super::super::tempfile::tempdir().unwrap();
        let (args, db, release) = fixture(root.path());
        retry_with(
            &db,
            &args,
            root.path(),
            &release,
            |_| fresh(&release),
            |r| {
                assert_eq!(fs::read(&r.source).unwrap(), b"fresh bytes");
                Ok(())
            },
        )
        .unwrap();
        let parked = db
            .query("SELECT path,no_burn FROM parked_payloads", &[])
            .unwrap();
        assert_eq!(parked.len(), 1);
        assert_eq!(parked[0][1].as_deref(), Some("1"));
        let path = parked[0][0].as_deref().unwrap();
        assert_eq!(fs::read(path).unwrap(), b"bad archive evidence");
        assert_eq!(
            db.query("SELECT parked,crc_reacquired FROM releases", &[])
                .unwrap()[0],
            vec![Some("0".into()), Some("1".into())]
        );
        // Even an accidental successful-history reference cannot burn the parked copy.
        db.query(
            "UPDATE releases SET file_path=?,process_date='done',processed_path='/unused'",
            &[path],
        )
        .unwrap();
        assert!(super::super::clean::completed(&db, &args, root.path(), None).is_err());
        assert!(Path::new(path).exists());
    }

    #[test]
    fn second_crc_failure_is_durable_and_not_selected_even_with_force() {
        let root = super::super::tempfile::tempdir().unwrap();
        let (mut args, db, release) = fixture(root.path());
        let result: Result<()> = retry_with(
            &db,
            &args,
            root.path(),
            &release,
            |_| fresh(&release),
            |_| Err(crate::archive::CrcFailure.into()),
        );
        assert!(result.unwrap_err().is::<crate::archive::CrcFailure>());
        assert_eq!(
            db.query("SELECT path FROM parked_payloads WHERE no_burn=1", &[])
                .unwrap()
                .len(),
            2
        );
        assert!(!release.source.exists());
        drop(db);
        let db = Database::open(root.path().join("collection.sqlite3").to_str().unwrap()).unwrap();
        args.force = true;
        assert!(
            super::super::processing_rows(&db, &args)
                .unwrap()
                .is_empty()
        );
        assert!(db.completed("johnsmith", "album1", true).unwrap());
        assert_eq!(
            db.query("SELECT parked,crc_reacquired FROM releases", &[])
                .unwrap()[0],
            vec![Some("1".into()), Some("1".into())]
        );
    }

    #[test]
    fn offline_and_failed_acquisition_park_without_automatic_loops() {
        for offline in [false, true] {
            let root = super::super::tempfile::tempdir().unwrap();
            let (mut args, db, release) = fixture(root.path());
            args.offline = offline;
            let result: Result<()> = retry_with(
                &db,
                &args,
                root.path(),
                &release,
                |_| {
                    assert!(!offline);
                    anyhow::bail!("network unavailable")
                },
                |_| panic!("Must not process without a replacement"),
            );
            assert!(result.is_err());
            assert!(
                super::super::processing_rows(&db, &args)
                    .unwrap()
                    .is_empty()
            );
            let parked = db.query("SELECT path FROM parked_payloads", &[]).unwrap();
            assert_eq!(
                fs::read(parked[0][0].as_deref().unwrap()).unwrap(),
                b"bad archive evidence"
            );
        }
    }
}
