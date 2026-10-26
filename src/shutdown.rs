use anyhow::{Result, ensure};
use std::{
    future::Future,
    sync::atomic::{AtomicI32, Ordering},
    time::{Duration, Instant},
};

static SIGNAL: AtomicI32 = AtomicI32::new(0);

pub fn install() -> Result<()> {
    for signal in [libc::SIGINT, libc::SIGTERM] {
        // The signal callback only writes an atomic; cleanup runs on ordinary threads.
        unsafe {
            signal_hook_registry::register(signal, move || {
                let _ = SIGNAL.compare_exchange(0, signal, Ordering::Relaxed, Ordering::Relaxed);
            })?;
        }
    }
    Ok(())
}
pub fn requested() -> bool {
    SIGNAL.load(Ordering::Relaxed) != 0
}
pub fn exit_code() -> i32 {
    128 + SIGNAL.load(Ordering::Relaxed)
}
pub fn check() -> Result<()> {
    ensure!(!requested(), "Termination requested");
    Ok(())
}
pub fn pause(duration: Duration) -> Result<()> {
    let end = Instant::now() + duration;
    loop {
        check()?;
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Ok(());
        }
        std::thread::sleep(left.min(Duration::from_millis(50)));
    }
}
pub async fn until<T>(future: impl Future<Output = T>) -> Result<T> {
    let canceled = async {
        while !requested() {
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    };
    tokio::select! {
        biased;
        _ = canceled => anyhow::bail!("Termination requested"),
        result = future => Ok(result),
    }
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use crate::db::Database;
    use std::{
        fs,
        io::{self, Read},
        path::Path,
        process::{Command, Stdio},
    };

    struct InterruptedReader<'a> {
        root: &'a Path,
        header: bool,
    }
    impl Read for InterruptedReader<'_> {
        fn read(&mut self, out: &mut [u8]) -> io::Result<usize> {
            if !self.header {
                self.header = true;
                out[..12].copy_from_slice(b"fLaC12345678");
                return Ok(12);
            }
            fs::write(self.root.join("ready"), b"ready")?;
            pause(Duration::from_secs(60)).map_err(io::Error::other)?;
            Ok(0)
        }
    }
    #[test]
    #[ignore = "subprocess fixture; invoked by termination_signals_cleanup"]
    fn termination_fixture() {
        let root = std::path::PathBuf::from(
            std::env::var_os("BCFUNL_SHUTDOWN_FIXTURE").expect("fixture directory"),
        );
        install().unwrap();
        let db = Database::open(root.join("collection.sqlite3").to_str().unwrap()).unwrap();
        for key in ["done", "pending"] {
            db.query("INSERT INTO releases(user,item_key,artist,release_title,download_url) VALUES('fixture',?,'Artist','Title','https://bandcamp.com/download?fixture')",&[key]).unwrap();
        }
        db.query(
            "UPDATE releases SET download_date='2025-01-01' WHERE item_key='done'",
            &[],
        )
        .unwrap();
        let result = match std::env::var("BCFUNL_SHUTDOWN_MODE").unwrap().as_str() {
            "aria2" => crate::aria2::cancellation_fixture(&root),
            "processing" => crate::processing::cancellation_fixture(&root),
            "save" => crate::download::save(
                InterruptedReader {
                    root: &root,
                    header: false,
                },
                None,
                &root,
                "song",
                "flac",
                false,
            )
            .map(|_| ()),
            "async" => {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                fs::write(root.join("ready"), b"ready").unwrap();
                runtime.block_on(until(std::future::pending::<()>()))
            }
            "parallel" => {
                use clap::Parser;
                let args =
                    crate::Args::try_parse_from(["bcfunl", "--user", "fixture", "--parallel", "2"])
                        .unwrap();
                let releases: Vec<_> = ["done", "pending"]
                    .into_iter()
                    .map(|key| crate::Release {
                        key: key.into(),
                        artist: "Artist".into(),
                        title: "Title".into(),
                        url: "https://bandcamp.com/download?fixture".into(),
                        publish: None,
                        release_url: None,
                        release_custom_host: None,
                        private: false,
                    })
                    .collect();
                crate::process_parallel(
                    &db,
                    &args,
                    &releases,
                    |r| Ok(r.clone()),
                    |_| {
                        crate::aria2::cancellation_fixture(&root)?;
                        anyhow::bail!("Fixture child unexpectedly exited")
                    },
                )
            }
            "pause" => {
                fs::write(root.join("ready"), b"ready").unwrap();
                pause(Duration::from_secs(60))
            }
            _ => panic!("Unknown fixture mode"),
        };
        assert!(result.is_err());
        assert!(requested(), "fixture failed before signal: {result:?}");
        db.dump(&mut fs::File::create(root.join("table.tsv")).unwrap(), None)
            .unwrap();
        drop(db);
        std::process::exit(exit_code());
    }
    #[test]
    fn termination_signals_cleanup() {
        for signal in [libc::SIGINT, libc::SIGTERM] {
            for mode in ["pause", "save", "aria2", "async", "parallel", "processing"] {
                let root = crate::tests::tempdir();
                let mut child = Command::new(std::env::current_exe().unwrap())
                    .args([
                        "--ignored",
                        "--exact",
                        "shutdown::tests::termination_fixture",
                        "--nocapture",
                    ])
                    .env("BCFUNL_SHUTDOWN_FIXTURE", &root)
                    .env("BCFUNL_SHUTDOWN_MODE", mode)
                    .stdout(Stdio::null())
                    .spawn()
                    .unwrap();
                let deadline = Instant::now() + Duration::from_secs(10);
                while !root.join("ready").exists() {
                    if let Some(status) = child.try_wait().unwrap() {
                        panic!("{mode} fixture exited early: {status}");
                    }
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        panic!("{mode} fixture startup timed out");
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
                let deadline = Instant::now() + Duration::from_secs(5);
                let status = loop {
                    if let Some(status) = child.try_wait().unwrap() {
                        break status;
                    }
                    if Instant::now() > deadline {
                        let _ = child.kill();
                        let _ = child.wait();
                        panic!("{mode} cleanup timed out");
                    }
                    std::thread::sleep(Duration::from_millis(10));
                };
                assert_eq!(status.code(), Some(128 + signal), "{mode}");
                assert!(!root.join("song [flac].flac.part").exists(), "{mode}");
                assert!(!root.join(".aria2-fixture").exists(), "{mode}");
                assert!(
                    !fs::read_dir(&root).unwrap().any(|entry| entry
                        .unwrap()
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".bcfunl-process-")),
                    "{mode}"
                );
                if mode == "aria2" || mode == "parallel" || mode == "processing" {
                    let pid = fs::read_to_string(root.join("child-pid"))
                        .unwrap()
                        .trim()
                        .parse::<i32>()
                        .unwrap();
                    assert_eq!(unsafe { libc::kill(pid, 0) }, -1, "Child was not reaped");
                    assert_eq!(io::Error::last_os_error().raw_os_error(), Some(libc::ESRCH));
                }
                let db = Database::open(root.join("collection.sqlite3").to_str().unwrap()).unwrap();
                assert_eq!(
                    db.query("PRAGMA integrity_check", &[]).unwrap()[0][0].as_deref(),
                    Some("ok")
                );
                assert!(db.completed("fixture", "done", false).unwrap());
                assert!(!db.completed("fixture", "pending", false).unwrap());
                assert_eq!(
                    fs::read_to_string(root.join("table.tsv"))
                        .unwrap()
                        .lines()
                        .count(),
                    4
                );
                drop(db);
                fs::remove_dir_all(root).unwrap();
            }
        }
    }
}
