use anyhow::{Context, Result, ensure};
use std::{
    fs::{File, OpenOptions},
    path::Path,
};

pub(crate) struct StagingLock(File);
impl Drop for StagingLock {
    fn drop(&mut self) {
        let _ = self.0.unlock();
    }
}

pub(crate) fn lock(input: &Path) -> Result<StagingLock> {
    #[cfg(unix)]
    use std::os::unix::fs::OpenOptionsExt;
    #[cfg(windows)]
    use std::os::windows::fs::OpenOptionsExt;
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(false);
    #[cfg(unix)]
    options.custom_flags(libc::O_NOFOLLOW);
    #[cfg(windows)]
    options.custom_flags(0x00200000); // Open the reparse point itself, never its target.
    let lock = options.open(input.join(".bcfunl-staging.lock"))?;
    ensure!(
        !lock.metadata()?.file_type().is_symlink(),
        "Staging lock is a symlink"
    );
    lock.try_lock()
        .context("Another download, process, or clean job holds the staging directory lock")?;
    Ok(StagingLock(lock))
}
