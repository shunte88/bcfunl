//! Delete only recorded staged payloads with successful, still-present publications.
use super::{completed_here, processing_rows, staging_lock};
use crate::{Args, db::Database, shutdown};
use anyhow::{Context, Result, ensure};
use std::{fs, path::Path};

pub(crate) fn run(args: &Args) -> Result<()> {
    let input = args
        .output
        .canonicalize()
        .context("Open existing --output staging directory")?;
    let _lock = staging_lock(&input)?;
    let path = input.join("collection.sqlite3");
    ensure!(
        path.is_file(),
        "--clean requires an existing collection.sqlite3 in --output"
    );
    let db = Database::open(path.to_str().context("Database path is not UTF-8")?)?;
    completed(&db, args, &input, None)
}

fn remove(db: &Database, args: &Args, input: &Path, row: &crate::db::Row) -> Result<bool> {
    let get = |i: usize| row[i].as_deref().unwrap_or_default();
    let source = Path::new(get(3));
    match fs::symlink_metadata(source) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        result => {
            result.context("Inspect staged payload")?;
        }
    }
    // Reject symlinks anywhere in the recorded path, including links inside staging.
    for ancestor in source.ancestors().filter(|p| !p.as_os_str().is_empty()) {
        ensure!(
            !fs::symlink_metadata(ancestor)?.file_type().is_symlink(),
            "Staged payload path contains a symlink"
        );
    }
    let source = source.canonicalize()?;
    let user_root = input.join(&args.user);
    ensure!(
        source.starts_with(&user_root),
        "Staged payload is outside the user's staging directory"
    );
    ensure!(
        source.is_file() && (super::audio(&source) || super::extension(&source) == "zip"),
        "Not a supported staged payload file"
    );
    let destination = Path::new(get(10));
    let server = destination
        .canonicalize()
        .context("Published release is unavailable; retain source")?;
    ensure!(
        !server.starts_with(input) && !input.starts_with(&server),
        "Published release and staging must be separate trees"
    );
    ensure!(
        completed_here(get(10), &server, get(0)),
        "Published manifest/files do not verify; retain source"
    );
    let references = db.query(
        "SELECT user,item_key FROM releases WHERE file_path=?",
        &[get(3)],
    )?;
    ensure!(
        references.len() == 1,
        "Payload is referenced by multiple releases; retain source"
    );
    shutdown::check()?;
    fs::remove_file(&source).context("Remove processed staged payload")?;
    eprintln!("  Cleaned {}", source.display());
    Ok(true)
}

pub(super) fn completed(
    db: &Database,
    args: &Args,
    input: &Path,
    keys: Option<&std::collections::HashSet<String>>,
) -> Result<()> {
    let mut cleaned = 0;
    let mut failures = 0;
    for row in processing_rows(db, args)? {
        shutdown::check()?;
        if keys.is_some_and(|keys| !keys.contains(row[0].as_deref().unwrap_or_default())) {
            continue;
        }
        if args.count.is_some_and(|limit| cleaned >= limit) {
            break;
        }
        if row[9].as_deref().is_none_or(str::is_empty) || row[11].is_some() {
            continue;
        }
        // Keep the success-status check stable until the payload removal finishes.
        db.query("BEGIN IMMEDIATE", &[])?;
        let result = (|| {
            let current = db.query("SELECT 1 FROM releases WHERE user=? AND item_key=? AND file_path=? AND download_date IS NOT NULL AND last_error IS NULL AND process_date IS NOT NULL AND process_error IS NULL AND processed_path=?", &[&args.user,row[0].as_deref().unwrap_or_default(),row[3].as_deref().unwrap_or_default(),row[10].as_deref().unwrap_or_default()])?;
            if current.is_empty() {
                return Ok(false);
            }
            remove(db, args, input, &row)
        })();
        db.query("COMMIT", &[])?;
        match result {
            Ok(true) => cleaned += 1,
            Ok(false) => {}
            Err(error) if shutdown::requested() => return Err(error),
            Err(error) => {
                failures += 1;
                eprintln!(
                    "  Retained {} - {}: {error:#}",
                    row[1].as_deref().unwrap_or_default(),
                    row[2].as_deref().unwrap_or_default()
                );
            }
        }
    }
    println!("Cleanup: {cleaned} payloads removed, {failures} retained for review");
    ensure!(
        failures == 0,
        "{failures} processed payloads could not be safely cleaned"
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    use std::os::unix::fs::symlink;

    #[test]
    fn cleanup_requires_success_manifest_scope_and_exclusive_payload() {
        let root = super::super::tempfile::tempdir().unwrap();
        let input = root.path().join("input");
        let user = input.join("fixture/Artist");
        fs::create_dir_all(&user).unwrap();
        let args = Args::try_parse_from([
            "bcfunl",
            "--clean",
            "--user",
            "fixture",
            "--output",
            input.to_str().unwrap(),
        ])
        .unwrap();
        let db = Database::open(input.join("collection.sqlite3").to_str().unwrap()).unwrap();
        for (key, success, error, manifest) in [
            ("good", true, false, true),
            ("pending", false, false, true),
            ("failed", true, true, true),
            ("offline", true, false, false),
            ("link", true, false, true),
            ("outside", true, false, true),
            ("shared", true, false, true),
            ("missing_track", true, false, true),
            ("wrong_manifest", true, false, true),
        ] {
            let source = if key == "outside" {
                root.path().join("outside.zip")
            } else {
                user.join(format!("{key}.zip"))
            };
            fs::write(&source, b"source").unwrap();
            if key == "link" {
                fs::remove_file(&source).unwrap();
                symlink(user.join("good.zip"), &source).unwrap();
            }
            let dest = root.path().join("server").join(key);
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("01.flac"), b"published").unwrap();
            if manifest {
                fs::write(
                    dest.join(".bcfunl-release.json"),
                    serde_json::json!({"item_key":key,"files":["01.flac"]}).to_string(),
                )
                .unwrap();
            }
            if key == "missing_track" {
                fs::remove_file(dest.join("01.flac")).unwrap();
            }
            if key == "wrong_manifest" {
                fs::write(
                    dest.join(".bcfunl-release.json"),
                    r#"{"item_key":"other","files":["01.flac"]}"#,
                )
                .unwrap();
            }
            db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,download_date,file_path,process_date,process_error,processed_path) VALUES('fixture',?,'Artist',?,'unused','complete',?,NULLIF(?,''),NULLIF(?,''),?)", &[key,key,source.to_str().unwrap(),if success {"success"} else {""},if error {"failed"} else {""},dest.to_str().unwrap()]).unwrap();
        }
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,file_path) SELECT 'other','duplicate','Artist','Other','unused',file_path FROM releases WHERE item_key='shared'", &[]).unwrap();
        assert!(run(&args).is_err());
        assert!(!user.join("good.zip").exists());
        for key in [
            "pending",
            "failed",
            "offline",
            "shared",
            "missing_track",
            "wrong_manifest",
        ] {
            assert!(user.join(format!("{key}.zip")).exists());
        }
        assert!(
            fs::symlink_metadata(user.join("link.zip"))
                .unwrap()
                .file_type()
                .is_symlink()
        );
        assert!(root.path().join("outside.zip").exists());
        assert!(root.path().join("server/good/01.flac").exists());
        assert_eq!(
            db.query(
                "SELECT download_date,process_date,file_path FROM releases WHERE item_key='good'",
                &[]
            )
            .unwrap()[0][0]
                .as_deref(),
            Some("complete")
        );
        let lock = staging_lock(&input).unwrap();
        assert!(run(&args).is_err());
        drop(lock);
    }

    #[test]
    fn clean_modes_parse_and_resources_cannot_clean() {
        assert!(Args::try_parse_from(["bcfunl", "--clean"]).is_ok());
        assert!(
            Args::try_parse_from([
                "bcfunl",
                "--process",
                "--clean",
                "--parallel",
                "2",
                "--serverdir",
                "server"
            ])
            .is_ok()
        );
        assert!(Args::try_parse_from(["bcfunl", "--resources", "--clean"]).is_err());
        assert!(
            Args::try_parse_from(["bcfunl", "--clean", "--force"])
                .unwrap()
                .validate()
                .is_err()
        );
    }
}
