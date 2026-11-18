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

// Join whichever worker finishes first, keeping admission independent of launch order.
pub(crate) fn join_next<'scope, T, R>(
    active: &mut std::collections::VecDeque<(T, std::thread::ScopedJoinHandle<'scope, R>)>,
) -> Option<(T, std::thread::Result<R>)> {
    while !active.is_empty() {
        if let Some(index) = active.iter().position(|(_, task)| task.is_finished()) {
            let (job, task) = active.remove(index).unwrap();
            return Some((job, task.join()));
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finished_worker_admits_replacement_before_slow_first_worker() {
        std::thread::scope(|scope| {
            let (release, wait) = std::sync::mpsc::channel();
            let mut active = std::collections::VecDeque::new();
            active.push_back((
                0,
                scope.spawn(move || {
                    wait.recv_timeout(std::time::Duration::from_secs(5))
                        .expect("slow worker blocked refill");
                    0
                }),
            ));
            active.push_back((1, scope.spawn(|| 1)));
            let (id, result) = join_next(&mut active).unwrap();
            assert_eq!((id, result.unwrap()), (1, 1));
            active.push_back((
                2,
                scope.spawn(move || {
                    release.send(()).unwrap();
                    2
                }),
            ));
            let mut completed = Vec::new();
            while let Some((id, result)) = join_next(&mut active) {
                assert_eq!(id, result.unwrap());
                completed.push(id);
            }
            completed.sort();
            assert_eq!(completed, [0, 2]);
            assert!(join_next(&mut active).is_none());
        });
    }
    #[test]
    fn worker_panic_is_returned_to_the_coordinator() {
        std::thread::scope(|scope| {
            let mut active =
                std::collections::VecDeque::from([(1, scope.spawn(|| panic!("fixture")))]);
            assert!(join_next(&mut active).unwrap().1.is_err());
        });
    }
}
