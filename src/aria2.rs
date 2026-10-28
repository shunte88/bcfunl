use crate::{
    Args,
    download::{Downloaded, InvalidPayload, Prepared},
    net, shutdown,
};
use anyhow::{Context, Result, ensure};
use std::{
    fs::{self, File, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    time::Duration,
};

pub(crate) struct ChildGuard(pub(crate) Child);
impl ChildGuard {
    pub(crate) fn wait(&mut self) -> Result<ExitStatus> {
        loop {
            shutdown::check()?;
            if let Some(status) = self.0.try_wait().context("Poll child process")? {
                return Ok(status);
            }
            shutdown::pause(Duration::from_millis(50))?;
        }
    }
}
impl Drop for ChildGuard {
    fn drop(&mut self) {
        // Reap before deleting staging files, including on cancellation and unwinding.
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}
pub fn check() -> Result<()> {
    shutdown::check()?;
    let child = Command::new("aria2c")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("--parallel requires a working aria2c executable on PATH")?;
    let status = ChildGuard(child).wait()?;
    ensure!(
        status.success(),
        "aria2c cannot start; check its installation (including Snap confinement)"
    );
    Ok(())
}

fn command(dir: &Path) -> Command {
    let mut cmd = Command::new("aria2c");
    // The Rust scheduler bounds releases; aria2 bounds connections within each payload.
    cmd.args([
        "--no-conf=true",
        "--quiet=true",
        "--summary-interval=0",
        "--max-concurrent-downloads=16",
        "--max-connection-per-server=16",
        "--split=20",
        "--connect-timeout=30",
        "--timeout=90",
        "--continue=true",
        "--allow-overwrite=false",
        "--auto-file-renaming=false",
        "--file-allocation=none",
        "--follow-metalink=false",
        "--follow-torrent=false",
        "--check-certificate=true",
        "--input-file=-",
    ])
    .arg("--load-cookies")
    .arg(dir.join("cookies.txt"))
    .arg("--dir")
    .arg(dir)
    .stdin(Stdio::piped())
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    cmd
}
struct CookieFile(PathBuf);
impl Drop for CookieFile {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.0);
    }
}
fn cookie_file(source: &Path, dir: &Path) -> Result<CookieFile> {
    let text = fs::read_to_string(source).context("Read cookies for aria2c")?;
    let path = dir.join("cookies.txt");
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options
        .open(&path)
        .context("Create protected aria2c cookie export")?;
    let guard = CookieFile(path);
    writeln!(file, "# Netscape HTTP Cookie File")?;
    for line in text.lines() {
        let row = line.strip_prefix("#HttpOnly_").unwrap_or(line);
        if row.starts_with('#') || row.is_empty() {
            continue;
        }
        let fields: Vec<_> = row.split('\t').collect();
        ensure!(fields.len() == 7, "Invalid Netscape cookie row");
        let domain = fields[0].trim_start_matches('.');
        if domain == "bandcamp.com" || domain.ends_with(".bandcamp.com") {
            writeln!(file, "{line}")?;
        }
    }
    Ok(guard)
}
struct Staging {
    path: PathBuf,
    preserve: bool,
}
impl Drop for Staging {
    fn drop(&mut self) {
        if !self.preserve {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn transfer(cmd: &mut Command, url: &str) -> Result<()> {
    let url = reqwest::Url::parse(url).context("Invalid audio URL")?;
    ensure!(net::allowed(&url, true), "Unexpected audio URL origin");
    shutdown::check()?;
    let mut child = ChildGuard(cmd.spawn().context("Start aria2c")?);
    // Send signed URLs over stdin so they do not appear in command arguments or logs.
    let input = (|| -> Result<()> {
        let mut stdin = child.0.stdin.take().context("Missing aria2c stdin")?;
        writeln!(stdin, "{url}\n  out=payload").context("Send download to aria2c")?;
        Ok(())
    })();
    input?;
    let status = child.wait()?;
    ensure!(status.success(), "aria2c transfer failed ({status})");
    Ok(())
}
fn downloads_home() -> Result<PathBuf> {
    let home =
        std::env::var_os("HOME").context("HOME is required for aria2c staging in ~/Downloads")?;
    let home = PathBuf::from(home);
    ensure!(home.is_absolute(), "HOME must be an absolute path");
    Ok(home)
}
fn staging_owner(name: &str) -> Option<i32> {
    let (pid, token) = name.strip_prefix("bcfunl-")?.split_once('-')?;
    if token.len() != 16 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let pid = pid.parse::<i32>().ok()?;
    (pid > 0).then_some(pid)
}
#[cfg(target_os = "linux")]
fn active_transfer_directories() -> Result<std::collections::HashSet<PathBuf>> {
    use std::os::unix::ffi::OsStrExt;
    let mut active = std::collections::HashSet::new();
    for process in fs::read_dir("/proc").context("Inspect active transfer processes")? {
        let process = process?;
        if !process
            .file_name()
            .to_string_lossy()
            .bytes()
            .all(|b| b.is_ascii_digit())
        {
            continue;
        }
        let Ok(command) = fs::read(process.path().join("cmdline")) else {
            continue;
        };
        let args: Vec<_> = command.split(|b| *b == 0).collect();
        for pair in args.windows(2) {
            if pair[0] == b"--dir" {
                active.insert(PathBuf::from(std::ffi::OsStr::from_bytes(pair[1])));
            }
        }
    }
    Ok(active)
}
fn cleanup(home: &Path) -> Result<usize> {
    let downloads = home.join("Downloads");
    if !downloads.exists() {
        return Ok(0);
    }
    #[cfg(target_os = "linux")]
    let active = active_transfer_directories()?;
    let mut removed = 0;
    for entry in fs::read_dir(&downloads).context("Read Downloads staging directories")? {
        shutdown::check()?;
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let Some(pid) = entry.file_name().to_str().and_then(staging_owner) else {
            continue;
        };
        #[cfg(target_os = "linux")]
        if active.contains(&entry.path()) {
            continue;
        }
        // A live or inaccessible owner may still be writing; leave its entire directory alone.
        if unsafe { libc::kill(pid, 0) } == 0
            || std::io::Error::last_os_error().raw_os_error() != Some(libc::ESRCH)
        {
            continue;
        }
        fs::remove_dir_all(entry.path()).with_context(|| {
            format!("Remove stale staging directory {}", entry.path().display())
        })?;
        removed += 1;
    }
    Ok(removed)
}
pub fn cleanup_stale() -> Result<()> {
    let removed = cleanup(&downloads_home()?)?;
    if removed > 0 {
        eprintln!("Removed {removed} stale bcfunl staging directories from ~/Downloads.");
    }
    Ok(())
}
fn staging(home: &Path) -> Result<Staging> {
    ensure!(home.is_absolute(), "HOME must be an absolute path");
    // Keep the visible Downloads path: Snap confinement may reject its bind-mount backing path.
    let downloads = home.join("Downloads");
    fs::create_dir_all(&downloads).context("Create ~/Downloads")?;
    let path = downloads.join(format!(
        "bcfunl-{}-{:016x}",
        std::process::id(),
        rand::random::<u64>()
    ));
    fs::create_dir(&path).context("Create aria2c staging directory in ~/Downloads")?;
    Ok(Staging {
        path,
        preserve: false,
    })
}
fn publish(staging: &mut Staging, prepared: &Prepared, force: bool) -> Result<Downloaded> {
    let payload = staging.path.join("payload");
    // The shared saver supports different filesystems and records completion only after publication.
    let result = (|| {
        let file = File::open(&payload).context("aria2c did not produce a download")?;
        let bytes = file.metadata()?.len();
        prepared.save(file, Some(bytes), force)
    })();
    if result
        .as_ref()
        .is_err_and(|error| !error.is::<InvalidPayload>())
        && !shutdown::requested()
    {
        staging.preserve = true;
        eprintln!("Downloaded payload retained at {}", payload.display());
    }
    result
}
pub fn fetch(args: &Args, prepared: Prepared) -> Result<Downloaded> {
    shutdown::check()?;
    let mut staging = staging(&downloads_home()?)?;
    let _cookies = cookie_file(&args.cookies, &staging.path)?;
    crate::download::with_recovery(
        prepared,
        |prepared| {
            transfer(&mut command(&staging.path), &prepared.url)?;
            ensure!(
                !staging.path.join("payload.aria2").exists(),
                "aria2c left an incomplete transfer"
            );
            match publish(&mut staging, prepared, args.force) {
                Ok(done) => Ok(crate::download::TransferResult::Complete(done)),
                Err(error) if error.is::<InvalidPayload>() => {
                    let html =
                        crate::download::read_html(File::open(staging.path.join("payload"))?)?;
                    fs::remove_file(staging.path.join("payload"))?;
                    Ok(crate::download::TransferResult::Page(html))
                }
                Err(error) => Err(error),
            }
        },
        |prepared, html| {
            let net = crate::net::Network::new(&args.cookies)?;
            // Recovery requests and the next transfer retain the randomized pacing.
            net.pace()?;
            net.pace()?;
            prepared.refresh(&net, html, args.format)?;
            net.pace()
        },
    )
}

#[cfg(test)]
pub(crate) fn cancellation_fixture(root: &Path) -> Result<()> {
    let staging_path = root.join(".aria2-fixture");
    fs::create_dir(&staging_path)?;
    let staging = Staging {
        path: staging_path,
        preserve: false,
    };
    fs::write(staging.path.join("payload"), b"partial")?;
    let child = Command::new("/bin/sleep").arg("60").spawn()?;
    let mut child = ChildGuard(child);
    fs::write(root.join("child-pid"), child.0.id().to_string())?;
    fs::write(root.join("ready"), b"ready")?;
    let result = child.wait().map(|_| ());
    drop(child);
    drop(staging);
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn command_limits_connections_and_keeps_urls_out_of_arguments() {
        let command = command(Path::new("a directory"));
        let args: Vec<_> = command.get_args().map(|s| s.to_str().unwrap()).collect();
        for expected in [
            "--split=20",
            "--max-connection-per-server=16",
            "--max-concurrent-downloads=16",
            "--input-file=-",
            "--no-conf=true",
            "--load-cookies",
            "--continue=true",
            "--allow-overwrite=false",
            "--auto-file-renaming=false",
        ] {
            assert!(args.contains(&expected));
        }
        assert_eq!(args.last(), Some(&"a directory"));
        assert!(!args.contains(&"--out=payload"));
        assert!(!args.iter().any(|s| s.contains("https://")));
    }
    #[test]
    fn aria2_cookie_export_is_scoped_private_and_removed() {
        let root = crate::tests::tempdir();
        let source = root.join("source.txt");
        fs::write(&source,".bandcamp.com\tTRUE\t/\tTRUE\t0\tsession\tfixture\nexample.com\tFALSE\t/\tTRUE\t0\tother\tother-fixture\n").unwrap();
        let export = cookie_file(&source, &root).unwrap();
        let text = fs::read_to_string(&export.0).unwrap();
        assert!(text.contains(".bandcamp.com"));
        assert!(!text.contains("example.com") && !text.contains("other-fixture"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&export.0).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        let path = export.0.clone();
        drop(export);
        assert!(!path.exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn stages_under_visible_downloads_and_cleans_only_owned_directory() {
        let home = crate::tests::tempdir();
        fs::create_dir(home.join("Downloads")).unwrap();
        fs::write(home.join("Downloads/keep.txt"), b"keep").unwrap();
        let stage = staging(&home).unwrap();
        assert_eq!(stage.path.parent(), Some(home.join("Downloads").as_path()));
        assert!(
            stage
                .path
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .starts_with("bcfunl-")
        );
        let path = stage.path.clone();
        fs::write(path.join("payload"), b"partial").unwrap();
        drop(stage);
        assert!(!path.exists());
        assert!(home.join("Downloads/keep.txt").exists());
        assert!(staging(Path::new("relative")).is_err());
        fs::remove_dir_all(home).unwrap();
    }
    #[test]
    fn lift_and_shift_removes_source_only_after_success() {
        let home = crate::tests::tempdir();
        let output = crate::tests::tempdir();
        let bytes = b"fLaCcomplete-fixture-payload";
        let mut stage = staging(&home).unwrap();
        let source = stage.path.clone();
        fs::write(source.join("payload"), bytes).unwrap();
        let done = publish(&mut stage, &Prepared::fixture(output.clone()), false).unwrap();
        assert!(done.path.starts_with(&output));
        assert_eq!(fs::read(&done.path).unwrap(), bytes);
        drop(stage);
        assert!(!source.exists());

        let mut stage = staging(&home).unwrap();
        let retained = stage.path.clone();
        fs::write(retained.join("payload"), bytes).unwrap();
        assert!(publish(&mut stage, &Prepared::fixture(output.clone()), false).is_err());
        drop(stage);
        assert_eq!(fs::read(retained.join("payload")).unwrap(), bytes);
        assert_eq!(fs::read(&done.path).unwrap(), bytes);
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(output).unwrap();
    }
    #[test]
    fn invalid_payload_is_deleted_instead_of_retained() {
        let home = crate::tests::tempdir();
        let output = crate::tests::tempdir();
        let mut stage = staging(&home).unwrap();
        let path = stage.path.clone();
        fs::write(
            path.join("payload"),
            b"<!doctype html><html>Download page</html>",
        )
        .unwrap();
        assert!(publish(&mut stage, &Prepared::fixture(output.clone()), false).is_err());
        assert!(!stage.preserve);
        drop(stage);
        assert!(!path.exists());
        fs::remove_dir_all(home).unwrap();
        fs::remove_dir_all(output).unwrap();
    }
    #[test]
    fn cleanup_removes_only_stale_owned_directories_and_leaves_active_and_unrelated_files() {
        let home = crate::tests::tempdir();
        let downloads = home.join("Downloads");
        fs::create_dir(&downloads).unwrap();
        let active = downloads.join(format!("bcfunl-{}-0123456789abcdef", std::process::id()));
        let stale = downloads.join("bcfunl-2147483647-0123456789abcdef");
        for path in [&active, &stale] {
            fs::create_dir(path).unwrap();
            fs::write(path.join("payload"), b"partial").unwrap();
        }
        fs::write(stale.join("payload.aria2"), b"broken control file").unwrap();
        fs::write(downloads.join("keep.txt"), b"keep").unwrap();
        let unrelated = downloads.join("bcfunl-not-a-staging-folder");
        fs::create_dir(&unrelated).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(
            &unrelated,
            downloads.join("bcfunl-2147483647-1111111111111111"),
        )
        .unwrap();
        assert_eq!(cleanup(&home).unwrap(), 1);
        assert!(!stale.exists());
        assert!(active.join("payload").exists());
        assert!(downloads.join("keep.txt").exists() && unrelated.exists());
        assert_eq!(cleanup(&home).unwrap(), 0);
        assert!(staging_owner("bcfunl-0-0123456789abcdef").is_none());
        fs::remove_dir_all(home).unwrap();
    }
    #[cfg(target_os = "linux")]
    #[test]
    fn cleanup_protects_a_transfer_even_when_its_parent_is_gone() {
        let home = crate::tests::tempdir();
        let downloads = home.join("Downloads");
        fs::create_dir(&downloads).unwrap();
        let path = downloads.join("bcfunl-2147483647-fedcba9876543210");
        fs::create_dir(&path).unwrap();
        fs::write(path.join("payload"), b"active").unwrap();
        let mut child = Command::new("/bin/sh")
            .args(["-c", "read ignored", "fixture", "--dir"])
            .arg(&path)
            .stdin(Stdio::piped())
            .spawn()
            .unwrap();
        let removed = cleanup(&home);
        let _ = child.kill();
        let _ = child.wait();
        assert_eq!(removed.unwrap(), 0);
        assert!(path.join("payload").exists());
        assert_eq!(cleanup(&home).unwrap(), 1);
        assert!(!path.exists());
        fs::remove_dir_all(home).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn input_file_sets_payload_name_per_download() {
        let root = crate::tests::tempdir();
        let path = root.join("aria2-input");
        let mut cmd = Command::new("/usr/bin/tee");
        cmd.arg(&path).stdin(Stdio::piped()).stdout(Stdio::null());
        transfer(&mut cmd, "https://bandcamp.com/download?fixture").unwrap();
        assert_eq!(
            fs::read_to_string(path).unwrap(),
            "https://bandcamp.com/download?fixture\n  out=payload\n"
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn failed_process_is_not_treated_as_success() {
        let mut cmd = Command::new("/bin/false");
        cmd.stdin(Stdio::piped());
        assert!(transfer(&mut cmd, "https://bandcamp.com/download?test=fixture").is_err());
        assert!(transfer(&mut cmd, "https://example.com/invalid").is_err());
    }
}
