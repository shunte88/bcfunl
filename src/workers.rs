use anyhow::Result;

// A shared bounded pool prevents release-level parallelism multiplying track workers.
pub(crate) fn file_pool() -> Result<&'static rayon::ThreadPool> {
    static POOL: std::sync::OnceLock<std::result::Result<rayon::ThreadPool, String>> =
        std::sync::OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(
                std::thread::available_parallelism()
                    .map(usize::from)
                    .unwrap_or(1)
                    .min(crate::limits::workers().unwrap_or(8)),
            )
            .build()
            .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(|e| {
        let message = format!("Create file worker pool: {e}");
        if crate::limits::workers().is_some() {
            crate::limits::stop(&message);
            anyhow::Error::new(crate::limits::ResourceLimit(message))
        } else {
            anyhow::anyhow!(message)
        }
    })
}
