//! Cooperative process budgets; Docker/cgroups provide hard enforcement.
use anyhow::{Context, Result, ensure};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

const WORKING_SET: u64 = 128 * 1024 * 1024;
const HEADROOM: u64 = 64 * 1024 * 1024;
static ACTIVE: OnceLock<Arc<Budget>> = OnceLock::new();

#[derive(Debug)]
pub(crate) struct ResourceLimit(pub String);
impl std::fmt::Display for ResourceLimit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Processing resource budget: {}", self.0)
    }
}
impl std::error::Error for ResourceLimit {}

struct Capacity {
    cpus: f64,
    memory: u64,
    available: u64,
    rss: u64,
}
struct State {
    wall: Instant,
    cpu: f64,
    credit: f64,
    sampled: Instant,
    capacity: Capacity,
    failure: Option<String>,
}
struct Budget {
    percent: u8,
    workers: usize,
    state: Mutex<State>,
}

pub(crate) struct Governor {
    stopped: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}
impl Drop for Governor {
    fn drop(&mut self) {
        self.stopped.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            thread.thread().unpark();
            let _ = thread.join();
        }
    }
}

fn kb(text: &str, key: &str) -> Result<u64> {
    let value = text
        .lines()
        .find_map(|line| line.strip_prefix(key))
        .context("Missing memory metric")?
        .split_whitespace()
        .next()
        .context("Empty memory metric")?
        .parse::<u64>()?;
    value.checked_mul(1024).context("Memory metric overflow")
}
fn quota(text: &str) -> Result<Option<f64>> {
    let mut fields = text.split_whitespace();
    let quota = fields.next().context("Missing CPU quota")?;
    let period = fields
        .next()
        .context("Missing CPU period")?
        .parse::<f64>()?;
    ensure!(period > 0., "Invalid CPU period");
    if quota == "max" {
        return Ok(None);
    }
    let quota = quota.parse::<f64>()?;
    ensure!(quota > 0., "Invalid CPU quota");
    Ok(Some(quota / period))
}
fn number(path: &Path) -> Result<Option<u64>> {
    match fs::read_to_string(path) {
        Ok(text) if text.trim() == "max" => Ok(None),
        Ok(text) => Ok(Some(text.trim().parse()?)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}
fn cgroup() -> Result<PathBuf> {
    let root = Path::new("/sys/fs/cgroup");
    let membership =
        fs::read_to_string("/proc/self/cgroup").context("Read process cgroup membership")?;
    let path = membership
        .lines()
        .find_map(|line| line.strip_prefix("0::"))
        .context("Processing governor requires cgroup v2")?;
    let joined = root.join(path.trim_start_matches('/'));
    if joined.is_dir() {
        Ok(joined)
    } else {
        ensure!(root.is_dir(), "Cannot inspect process cgroup limits");
        Ok(root.into())
    }
}
fn capacity() -> Result<Capacity> {
    ensure!(
        cfg!(target_os = "linux"),
        "--nice processing budgets currently require Linux"
    );
    let memory = fs::read_to_string("/proc/meminfo").context("Read available memory")?;
    let mut result = Capacity {
        cpus: std::thread::available_parallelism()?.get() as f64,
        memory: kb(&memory, "MemTotal:")?,
        available: kb(&memory, "MemAvailable:")?,
        rss: kb(&fs::read_to_string("/proc/self/status")?, "VmRSS:")?,
    };
    {
        let group = cgroup()?;
        for path in group
            .ancestors()
            .take_while(|p| p.starts_with("/sys/fs/cgroup"))
        {
            if let Some(max) = number(&path.join("memory.max"))? {
                result.memory = result.memory.min(max);
                if let Some(current) = number(&path.join("memory.current"))? {
                    let reclaimable = fs::read_to_string(path.join("memory.stat"))
                        .ok()
                        .and_then(|text| {
                            text.lines().find_map(|line| {
                                line.strip_prefix("inactive_file ")
                                    .and_then(|s| s.parse::<u64>().ok())
                            })
                        })
                        .unwrap_or(0);
                    result.available = result.available.min(
                        max.saturating_sub(current)
                            .saturating_add(reclaimable)
                            .min(max),
                    );
                }
            }
            match fs::read_to_string(path.join("cpu.max")) {
                Ok(text) => {
                    if let Some(max) = quota(&text)? {
                        result.cpus = result.cpus.min(max);
                    }
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
    }
    Ok(result)
}
fn cpu_time() -> Result<f64> {
    #[cfg(target_os = "linux")]
    {
        let mut time = libc::timespec {
            tv_sec: 0,
            tv_nsec: 0,
        };
        ensure!(
            unsafe { libc::clock_gettime(libc::CLOCK_PROCESS_CPUTIME_ID, &mut time) } == 0,
            "Read process CPU time"
        );
        Ok(time.tv_sec as f64 + time.tv_nsec as f64 / 1e9)
    }
    #[cfg(not(target_os = "linux"))]
    {
        anyhow::bail!("CPU accounting requires Linux")
    }
}
fn slots(c: &Capacity, percent: u8) -> usize {
    let memory = c.memory.saturating_mul(u64::from(percent)) / 100;
    let spare = memory
        .saturating_sub(c.rss)
        .min(c.available.saturating_sub(HEADROOM));
    ((c.cpus * f64::from(percent) / 100.).floor().max(1.) as usize)
        .min((spare / WORKING_SET) as usize)
        .min(8)
}
fn charge(credit: f64, cpu: f64, wall: f64, cores: f64) -> (f64, Duration) {
    let credit = (credit + cores * wall).min(cores * 0.1) - cpu;
    (
        credit,
        Duration::from_secs_f64((-credit / cores).clamp(0., 0.05)),
    )
}

impl Budget {
    fn refreshed(&self, state: &mut State) -> Result<()> {
        if state.sampled.elapsed() >= Duration::from_millis(250) {
            state.capacity = capacity()?;
            state.sampled = Instant::now();
        }
        let ceiling = state
            .capacity
            .memory
            .saturating_mul(u64::from(self.percent))
            / 100;
        if state.capacity.rss >= ceiling || state.capacity.available < HEADROOM {
            state.failure =
                Some("memory is below the safe processing budget; sources retained".into());
        }
        if let Some(failure) = &state.failure {
            return Err(ResourceLimit(failure.clone()).into());
        }
        Ok(())
    }
    fn pause(&self, state: &mut State) -> Result<Duration> {
        let cpu = cpu_time()?;
        let now = Instant::now();
        let (credit, pause) = charge(
            state.credit,
            cpu - state.cpu,
            now.duration_since(state.wall).as_secs_f64(),
            state.capacity.cpus * f64::from(self.percent) / 100.,
        );
        state.credit = credit;
        state.cpu = cpu;
        state.wall = now;
        Ok(pause)
    }
    fn checkpoint(&self) -> Result<()> {
        loop {
            crate::shutdown::check()?;
            let pause = {
                let mut state = self
                    .state
                    .lock()
                    .map_err(|_| anyhow::anyhow!("Resource accounting lock poisoned"))?;
                self.refreshed(&mut state)?;
                self.pause(&mut state)?
            };
            if pause.is_zero() {
                return Ok(());
            }
            crate::shutdown::pause(pause)?;
        }
    }
}
pub(crate) fn start(percent: u8) -> Result<Governor> {
    let c = capacity()?;
    let workers = slots(&c, percent);
    ensure!(
        workers > 0,
        "Insufficient memory for one processing job within --nice {percent}: {} MiB RAM budget, {} MiB RSS, {} MiB available; need a 128 MiB working reserve plus 64 MiB free headroom; sources untouched",
        c.memory * u64::from(percent) / 100 / 1024 / 1024,
        c.rss / 1024 / 1024,
        c.available / 1024 / 1024
    );
    eprintln!(
        "Processing budget: {percent}% of {:.2} CPU cores and {} MiB RAM; up to {workers} release/file workers (cooperative)",
        c.cpus,
        c.memory / 1024 / 1024
    );
    let now = Instant::now();
    let budget = Budget {
        percent,
        workers,
        state: Mutex::new(State {
            wall: now,
            cpu: cpu_time()?,
            credit: 0.,
            sampled: now,
            capacity: c,
            failure: None,
        }),
    };
    let budget = Arc::new(budget);
    ensure!(
        ACTIVE.set(budget.clone()).is_ok(),
        "Processing resource budget already initialized"
    );
    let stopped = Arc::new(AtomicBool::new(false));
    let stop = stopped.clone();
    let thread = std::thread::Builder::new()
        .name("bcfunl-governor".into())
        .spawn(move || {
            while !stop.load(Ordering::Relaxed) && !crate::shutdown::requested() {
                if let Ok(mut state) = budget.state.lock() {
                    match capacity() {
                        Ok(capacity) => {
                            state.capacity = capacity;
                            state.sampled = Instant::now();
                            if let Err(error) = budget
                                .refreshed(&mut state)
                                .and_then(|_| budget.pause(&mut state).map(|_| ()))
                            {
                                eprintln!("Governor: {error:#}");
                                break;
                            }
                        }
                        Err(error) => {
                            state.failure = Some(format!("resource monitoring failed: {error:#}"));
                            break;
                        }
                    }
                } else {
                    break;
                }
                std::thread::park_timeout(Duration::from_millis(250));
            }
        })
        .context("Start processing governor")?;
    Ok(Governor {
        stopped,
        thread: Some(thread),
    })
}
pub(crate) fn workers() -> Option<usize> {
    ACTIVE.get().map(|b| b.workers)
}
pub(crate) fn checkpoint() -> Result<()> {
    if let Some(budget) = ACTIVE.get()
        && let Err(error) = budget.checkpoint()
    {
        if crate::shutdown::requested() {
            return Err(error);
        }
        stop(&error.to_string());
        if error.is::<ResourceLimit>() {
            return Err(error);
        }
        return Err(ResourceLimit(error.to_string()).into());
    }
    Ok(())
}
pub(crate) fn admission() -> Result<usize> {
    let Some(budget) = ACTIVE.get() else {
        return Ok(8);
    };
    budget.checkpoint()?;
    let mut state = budget
        .state
        .lock()
        .map_err(|_| anyhow::anyhow!("Resource accounting lock poisoned"))?;
    state.capacity = capacity()?;
    state.sampled = Instant::now();
    budget.refreshed(&mut state)?;
    Ok(slots(&state.capacity, budget.percent).min(budget.workers))
}
pub(crate) fn exhausted() -> bool {
    ACTIVE.get().is_some_and(|budget| {
        budget
            .state
            .lock()
            .map_or(true, |state| state.failure.is_some())
    })
}
pub(crate) fn stop(reason: &str) {
    if let Some(budget) = ACTIVE.get()
        && let Ok(mut state) = budget.state.lock()
        && state.failure.is_none()
    {
        state.failure = Some(reason.into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pressure_is_sticky_and_reported_as_a_resource_failure() {
        let now = Instant::now();
        let budget = Budget {
            percent: 20,
            workers: 1,
            state: Mutex::new(State {
                wall: now,
                cpu: 0.,
                credit: 0.,
                sampled: now,
                capacity: Capacity {
                    cpus: 4.,
                    memory: 1024 * 1024 * 1024,
                    available: 1024 * 1024 * 1024,
                    rss: 512 * 1024 * 1024,
                },
                failure: None,
            }),
        };
        let mut state = budget.state.lock().unwrap();
        assert!(
            budget
                .refreshed(&mut state)
                .unwrap_err()
                .is::<ResourceLimit>()
        );
        state.capacity.rss = 0;
        assert!(budget.refreshed(&mut state).is_err());
    }

    #[test]
    fn budgets_reduce_workers_and_fail_below_a_working_set() {
        let mut c = Capacity {
            cpus: 4.,
            memory: 8 * 1024 * 1024 * 1024,
            available: 4 * 1024 * 1024 * 1024,
            rss: 64 * 1024 * 1024,
        };
        assert_eq!(slots(&c, 20), 1);
        c.cpus = 64.;
        assert_eq!(slots(&c, 20), 8);
        c.available = HEADROOM + WORKING_SET;
        assert_eq!(slots(&c, 50), 1);
        c.available -= 1;
        assert_eq!(slots(&c, 50), 0);
        assert_eq!(charge(0., 0.2, 0.1, 0.8).1, Duration::from_millis(50));
        assert!(charge(0., 0.01, 1., 0.8).1.is_zero());
        assert!(charge(0., 1., 1000., 0.8).0 < 0.); // Idle time cannot bank an unbounded CPU burst.
    }
    #[test]
    fn parses_linux_capacity_and_fractional_container_quota() {
        assert_eq!(kb("MemTotal: 1024 kB\n", "MemTotal:").unwrap(), 1024 * 1024);
        assert!(kb("", "MemTotal:").is_err());
        assert_eq!(quota("50000 100000").unwrap(), Some(0.5));
        assert_eq!(quota("max 100000").unwrap(), None);
        assert!(quota("50000 0").is_err());
    }
}
