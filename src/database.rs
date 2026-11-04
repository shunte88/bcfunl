//! Check, snapshot, and recover collection history under the shared staging lock.
use crate::{Args, db::Database, processing::tempfile, shutdown};
use anyhow::{Context, Result, bail, ensure};
use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use std::{
    fs,
    path::{Path, PathBuf},
};

const PREFIX: &str = "collection.sqlite3.";

fn private_directory(path: &Path) -> std::io::Result<()> {
    let mut builder = fs::DirBuilder::new();
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        builder.mode(0o700);
    }
    builder.create(path)
}

fn regular(path: &Path) -> Result<()> {
    ensure!(
        fs::symlink_metadata(path)?.file_type().is_file(),
        "Not a regular database file: {}",
        path.display()
    );
    Ok(())
}

fn check(path: &Path) -> Result<()> {
    regular(path)?;
    let db = Database::read_only(path.to_str().context("Database path is not UTF-8")?)?;
    ensure!(
        db.query("PRAGMA integrity_check", &[])? == vec![vec![Some("ok".into())]],
        "SQLite integrity check failed: {}",
        path.display()
    );
    ensure!(
        db.query("PRAGMA foreign_key_check", &[])?.is_empty(),
        "SQLite foreign key check failed"
    );
    let columns = db.query("PRAGMA table_info(releases)", &[])?;
    for required in [
        "user",
        "item_key",
        "artist",
        "release_title",
        "download_url",
        "download_date",
        "publish_date",
        "purchase_date",
    ] {
        ensure!(
            columns
                .iter()
                .any(|row| row[1].as_deref() == Some(required)),
            "Missing releases column: {required}"
        );
    }
    for (name, position) in [("user", "1"), ("item_key", "2")] {
        ensure!(
            columns
                .iter()
                .any(|row| row[1].as_deref() == Some(name) && row[5].as_deref() == Some(position)),
            "Unexpected releases primary key"
        );
    }
    let process_date = if columns
        .iter()
        .any(|row| row[1].as_deref() == Some("process_date"))
    {
        "process_date"
    } else {
        "NULL"
    };
    let sql = format!(
        "SELECT user,item_key,artist,release_title,publish_date,purchase_date,download_date,{process_date} FROM releases"
    );
    for row in db.query(&sql, &[])? {
        shutdown::check()?;
        let value = |i: usize| row[i].as_deref().unwrap_or_default();
        ensure!(
            !value(0).is_empty()
                && value(0)
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-'),
            "Invalid collection owner in history"
        );
        let key = value(1)
            .strip_prefix("album")
            .or_else(|| value(1).strip_prefix("track"));
        ensure!(
            key.is_some_and(|k| !k.is_empty() && k.bytes().all(|b| b.is_ascii_digit())),
            "Invalid release key in history"
        );
        ensure!(
            !value(2).trim().is_empty() && !value(3).trim().is_empty(),
            "Missing artist or release title in history"
        );
        for date in [4, 5] {
            if let Some(date) = &row[date] {
                ensure!(
                    NaiveDate::parse_from_str(date, "%Y-%m-%d").is_ok(),
                    "Invalid calendar date in history"
                );
            }
        }
        for date in [6, 7] {
            if let Some(date) = &row[date] {
                ensure!(
                    DateTime::parse_from_rfc3339(date).is_ok(),
                    "Invalid completion timestamp in history"
                );
            }
        }
    }
    Ok(())
}

fn backup_paths(directory: &Path) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    for entry in fs::read_dir(directory)? {
        let path = entry?.path();
        if path
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.strip_prefix(PREFIX))
            .is_some_and(|s| NaiveDateTime::parse_from_str(s, "%Y%m%dT%H%M%S%.fZ").is_ok())
        {
            paths.push(path);
        }
    }
    paths.sort();
    paths.reverse();
    Ok(paths)
}

fn snapshot(source: &Path, target: &Path) -> Result<()> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(target)?;
    Database::read_only(source.to_str().context("Source path is not UTF-8")?)?
        .backup_to(target.to_str().context("Backup path is not UTF-8")?)?;
    fs::File::open(target)?.sync_all()?;
    check(target)
}

fn migrate(path: &Path) -> Result<()> {
    drop(Database::open(
        path.to_str().context("Database path is not UTF-8")?,
    )?);
    check(path)
}

fn restore(source: &Path, live: &Path, input: &Path) -> Result<()> {
    let work = tempfile::tempdir_in(input)?;
    let replacement = work.path().join("collection.sqlite3");
    snapshot(source, &replacement)?;
    migrate(&replacement)?;
    // Keep all original files before removing sidecars or atomically replacing main.
    let evidence = input.join(format!(
        "database-quarantine-{}-{}",
        Utc::now().format("%Y%m%dT%H%M%SZ"),
        rand::random::<u64>()
    ));
    private_directory(&evidence)?;
    let mut originals = Vec::new();
    for suffix in ["", "-wal", "-shm", "-journal"] {
        let path = input.join(format!("collection.sqlite3{suffix}"));
        match fs::symlink_metadata(&path) {
            Ok(_) => {
                regular(&path)?;
                fs::hard_link(&path, evidence.join(path.file_name().unwrap()))?;
                originals.push(path);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    fs::File::open(&evidence)?.sync_all()?;
    shutdown::check()?;
    for path in originals.iter().filter(|p| p.as_path() != live) {
        fs::remove_file(path)?;
    }
    fs::rename(&replacement, live).context("Install verified restored database")?;
    fs::File::open(input)?.sync_all()?;
    check(live)?;
    println!(
        "Database restored from {}; originals retained in {}",
        source.display(),
        evidence.display()
    );
    Ok(())
}

pub(crate) fn run(args: &Args) -> Result<()> {
    let input = args
        .output
        .canonicalize()
        .context("Open existing --output staging directory")?;
    let _lock = crate::staging::lock(&input)?;
    let live = input.join("collection.sqlite3");
    if let Ok(metadata) = fs::symlink_metadata(&live) {
        ensure!(
            metadata.file_type().is_file(),
            "Live database must be a regular file"
        );
    }
    let backups = input.join("backups");
    match private_directory(&backups) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(e.into()),
    }
    ensure!(
        fs::symlink_metadata(&backups)?.file_type().is_dir(),
        "Backup directory must be a real directory"
    );
    if let Err(error) = check(&live) {
        eprintln!("Database check failed: {error:#}; searching verified backups");
        let mut restored = false;
        for backup in backup_paths(&backups)? {
            shutdown::check()?;
            match check(&backup) {
                Ok(()) => {
                    restore(&backup, &live, &input)?;
                    restored = true;
                    break;
                }
                Err(error) => eprintln!("Rejected backup {}: {error:#}", backup.display()),
            }
        }
        if !restored {
            bail!("No healthy database backup available; original history retained unchanged");
        }
    }
    // Capture a verified pre-migration snapshot so migrations are recoverable too.
    let work = tempfile::tempdir_in(&backups)?;
    let staged = work.path().join("snapshot.sqlite3");
    snapshot(&live, &staged)?;
    let target = backups.join(format!(
        "{PREFIX}{}",
        Utc::now().format("%Y%m%dT%H%M%S%.9fZ")
    ));
    fs::hard_link(&staged, &target).context("Publish backup without replacing existing backups")?;
    fs::File::open(&backups)?.sync_all()?;
    migrate(&live)?;
    let mut healthy = 0;
    for path in backup_paths(&backups)? {
        shutdown::check()?;
        match check(&path) {
            Ok(()) => {
                healthy += 1;
                if healthy > 7 {
                    fs::remove_file(&path)?;
                }
            }
            Err(error) => eprintln!(
                "Retained suspect backup {} for review: {error:#}",
                path.display()
            ),
        }
    }
    fs::File::open(&backups)?.sync_all()?;
    println!(
        "Database checks passed; backup {} (seven latest verified backups retained)",
        target.display()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;
    fn args(root: &Path) -> Args {
        Args::try_parse_from([
            "bcfunl",
            "--database",
            "--user",
            "johnsmith",
            "--output",
            root.to_str().unwrap(),
        ])
        .unwrap()
    }
    fn fixture(path: &Path) {
        let db = Database::open(path.to_str().unwrap()).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url,purchase_date,download_date) VALUES('johnsmith','album1','Artist','Title','https://bandcamp.com/download','2026-11-01','2026-11-01T00:00:00Z')",&[]).unwrap();
    }
    #[test]
    fn retains_seven_verified_backups_and_restores_past_a_bad_backup() {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("collection.sqlite3");
        fixture(&live);
        for _ in 0..9 {
            run(&args(root.path())).unwrap();
        }
        let folder = root.path().join("backups");
        assert_eq!(backup_paths(&folder).unwrap().len(), 7);
        let bad = folder.join("collection.sqlite3.20990101T000000.000000000Z");
        fs::write(&bad, b"bad backup").unwrap();
        fs::write(&live, b"damaged original").unwrap();
        run(&args(root.path())).unwrap();
        check(&live).unwrap();
        assert_eq!(
            Database::read_only(live.to_str().unwrap())
                .unwrap()
                .query("SELECT artist FROM releases", &[])
                .unwrap()[0][0]
                .as_deref(),
            Some("Artist")
        );
        assert_eq!(fs::read(&bad).unwrap(), b"bad backup");
        assert_eq!(backup_paths(&folder).unwrap().len(), 8);
        let quarantine = fs::read_dir(root.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .find(|p| {
                p.file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with("database-quarantine-")
            })
            .unwrap();
        assert_eq!(
            fs::read(quarantine.join("collection.sqlite3")).unwrap(),
            b"damaged original"
        );
    }
    #[test]
    fn refuses_missing_or_unrecoverable_history_and_invalid_semantics() {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("collection.sqlite3");
        assert!(run(&args(root.path())).is_err());
        assert!(!live.exists());
        fs::write(&live, b"original").unwrap();
        assert!(run(&args(root.path())).is_err());
        assert_eq!(fs::read(&live).unwrap(), b"original");
        fs::remove_file(&live).unwrap();
        fixture(&live);
        let db = Database::connect(live.to_str().unwrap()).unwrap();
        db.query("UPDATE releases SET download_date='bad-date'", &[])
            .unwrap();
        drop(db);
        assert!(check(&live).is_err());
    }
    #[test]
    fn checks_before_migration_and_keeps_wal_commits_in_native_backup() {
        let root = tempfile::tempdir().unwrap();
        let live = root.path().join("collection.sqlite3");
        let db = Database::connect(live.to_str().unwrap()).unwrap();
        db.query("CREATE TABLE releases(user TEXT NOT NULL,item_key TEXT NOT NULL,artist TEXT NOT NULL,release_title TEXT NOT NULL,publish_date TEXT,purchase_date TEXT,download_date TEXT,download_url TEXT NOT NULL,format TEXT,file_path TEXT,last_error TEXT,PRIMARY KEY(user,item_key))",&[]).unwrap();
        drop(db);
        run(&args(root.path())).unwrap();
        assert!(
            Database::read_only(live.to_str().unwrap())
                .unwrap()
                .query("PRAGMA table_info(releases)", &[])
                .unwrap()
                .iter()
                .any(|r| r[1].as_deref() == Some("parked"))
        );
        let db = Database::connect(live.to_str().unwrap()).unwrap();
        db.query("PRAGMA journal_mode=WAL", &[]).unwrap();
        db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url) VALUES('johnsmith','album1','Artist','WAL commit','https://bandcamp.com/download')",&[]).unwrap();
        let target = root.path().join("wal-backup.sqlite3");
        snapshot(&live, &target).unwrap();
        assert_eq!(
            Database::read_only(target.to_str().unwrap())
                .unwrap()
                .query("SELECT release_title FROM releases", &[])
                .unwrap()[0][0]
                .as_deref(),
            Some("WAL commit")
        );
    }
    #[test]
    fn database_mode_conflicts_and_staging_lock_are_enforced() {
        assert!(Args::try_parse_from(["bcfunl", "--database", "--process"]).is_err());
        assert!(Args::try_parse_from(["bcfunl", "--database", "--count", "1"]).is_err());
        let root = tempfile::tempdir().unwrap();
        fixture(&root.path().join("collection.sqlite3"));
        let _lock = crate::staging::lock(root.path()).unwrap();
        assert!(run(&args(root.path())).is_err());
    }
}
