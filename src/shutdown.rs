use anyhow::{Result, ensure};
use std::{
    future::Future,
    path::{Path, PathBuf},
    sync::{
        OnceLock,
        atomic::{AtomicI32, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

static SIGNAL: AtomicI32 = AtomicI32::new(0);

static STOP_FILE: OnceLock<PathBuf> = OnceLock::new();
static FILE_STATUS: AtomicI32 = AtomicI32::new(0);
static FILE_ERROR: OnceLock<String> = OnceLock::new();
static POLL_START: OnceLock<Instant> = OnceLock::new();
static NEXT_POLL: AtomicU64 = AtomicU64::new(0);

fn probe_file(path: &Path) {
    match std::fs::symlink_metadata(path) {
        Ok(_) => {
            FILE_STATUS.store(1, Ordering::Release);
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => {
            let _ = FILE_ERROR.set(format!(
                "Cannot inspect stop file {}: {error}",
                path.display()
            ));
            FILE_STATUS.store(2, Ordering::Release);
        }
    }
}
pub fn watch_file(path: PathBuf) -> Result<()> {
    let path = if path.is_absolute() {
        path
    } else {
        std::env::current_dir()?.join(path)
    };
    ensure!(STOP_FILE.set(path).is_ok(), "Stop file already configured");
    let _ = POLL_START.set(Instant::now());
    probe_file(STOP_FILE.get().unwrap());
    NEXT_POLL.store(50, Ordering::Relaxed);
    Ok(())
}
pub fn reason() -> String {
    if SIGNAL.load(Ordering::Relaxed) != 0 {
        return "Termination requested".into();
    }
    if let Some(error) = FILE_ERROR.get() {
        return error.clone();
    }
    STOP_FILE
        .get()
        .map(|p| format!("Stop file detected: {}", p.display()))
        .unwrap_or_else(|| "Termination requested".into())
}

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
    if SIGNAL.load(Ordering::Relaxed) != 0 || FILE_STATUS.load(Ordering::Acquire) != 0 {
        return true;
    }
    if let Some(path) = STOP_FILE.get() {
        let now = POLL_START.get_or_init(Instant::now).elapsed().as_millis() as u64;
        let next = NEXT_POLL.load(Ordering::Relaxed);
        if now >= next
            && NEXT_POLL
                .compare_exchange(
                    next,
                    now.saturating_add(50),
                    Ordering::Relaxed,
                    Ordering::Relaxed,
                )
                .is_ok()
        {
            probe_file(path);
        }
    }
    FILE_STATUS.load(Ordering::Acquire) != 0
}
pub fn exit_code() -> i32 {
    let signal = SIGNAL.load(Ordering::Relaxed);
    if signal != 0 {
        128 + signal
    } else if FILE_STATUS.load(Ordering::Acquire) == 2 {
        1
    } else {
        0
    }
}
pub fn check() -> Result<()> {
    ensure!(!requested(), "{}", reason());
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
        _ = canceled => anyhow::bail!("{}",reason()),
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
        if let Some(path) = std::env::var_os("BCFUNL_TEST_STOPFILE") {
            watch_file(path.into()).unwrap();
        }
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
        for signal in [0, libc::SIGINT, libc::SIGTERM] {
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
                    .env("BCFUNL_TEST_STOPFILE", root.join("stop"))
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
                if signal == 0 {
                    fs::write(root.join("stop"), b"stop").unwrap();
                } else {
                    assert_eq!(unsafe { libc::kill(child.id() as i32, signal) }, 0);
                }
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
                assert_eq!(
                    status.code(),
                    Some(if signal == 0 { 0 } else { 128 + signal }),
                    "{mode}"
                );
                if signal == 0 {
                    assert!(
                        root.join("stop").exists(),
                        "Stop file must remain for subsequent calls"
                    );
                }
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
