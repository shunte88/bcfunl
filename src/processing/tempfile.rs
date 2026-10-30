//! Private temporary paths, created exclusively and removed on drop.
use std::{
    fs, io,
    path::{Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub struct TempDir(PathBuf);
impl TempDir {
    pub fn path(&self) -> &Path {
        &self.0
    }
}
impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
#[cfg(test)]
pub fn tempdir() -> io::Result<TempDir> {
    tempdir_in(&std::env::temp_dir())
}
pub fn tempdir_in(parent: &Path) -> io::Result<TempDir> {
    #[cfg(unix)]
    use std::os::unix::fs::DirBuilderExt;
    for _ in 0..100 {
        let n = NEXT.fetch_add(1, Ordering::Relaxed);
        let p = parent.join(format!(".bcfunl-process-{}-{n}", std::process::id()));
        let mut builder = fs::DirBuilder::new();
        #[cfg(unix)]
        builder.mode(0o700);
        match builder.create(&p) {
            Ok(()) => return Ok(TempDir(p)),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(e),
        }
    }
    Err(io::Error::new(
        io::ErrorKind::AlreadyExists,
        "temporary names exhausted",
    ))
}
