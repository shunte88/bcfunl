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
                    .min(8),
            )
            .build()
            .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(|e| anyhow::anyhow!("Create file worker pool: {e}"))
}
