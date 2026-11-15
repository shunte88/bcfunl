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
    acquire(input, false)
}
#[cfg(unix)]
pub(crate) fn shared(input: &Path) -> Result<StagingLock> {
    acquire(input, true)
}
fn acquire(input: &Path, shared: bool) -> Result<StagingLock> {
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
    if shared { lock.try_lock_shared() } else { lock.try_lock() }.context(
        "Another download, process, clean, database, or incompatible collection job holds the staging directory lock",
    )?;
    Ok(StagingLock(lock))
}

#[cfg(all(test, unix))]
mod tests {
    #[test]
    fn collection_read_locks_exclude_maintenance_and_allow_peer_walkers() {
        let dir = crate::processing::tempfile::tempdir().unwrap();
        let a = super::shared(dir.path()).unwrap();
        let b = super::shared(dir.path()).unwrap();
        assert!(super::lock(dir.path()).is_err());
        drop(a);
        drop(b);
        let writer = super::lock(dir.path()).unwrap();
        assert!(super::shared(dir.path()).is_err());
        drop(writer);
        assert!(super::shared(dir.path()).is_ok());
    }
}
