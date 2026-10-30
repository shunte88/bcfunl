//! Native Windows downloads/resources are supported; media processing runs in Docker.
pub(crate) use crate::staging::lock as staging_lock;
#[path = "processing/tempfile.rs"]
pub(crate) mod tempfile;
pub(crate) fn run(_: &crate::Args) -> anyhow::Result<()> {
    anyhow::bail!(
        "Native Windows processing is not supported; use the Linux Docker Compose service"
    )
}
pub(crate) mod clean {
    pub(crate) fn run(_: &crate::Args) -> anyhow::Result<()> {
        anyhow::bail!(
            "Native Windows cleanup is not supported; use the Linux Docker Compose service"
        )
    }
}
