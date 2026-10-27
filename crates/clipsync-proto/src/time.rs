/// CLOCK_MONOTONIC in µs. Comparable across processes ONLY on the same machine (loopback): the
/// input loopback subtracts a sender's `t_mono_us` from the receiver's clock, so Unix keeps the
/// system-wide clock.
#[cfg(unix)]
pub fn mono_us() -> u64 {
    use rustix::time::{ClockId, clock_gettime};
    let ts = clock_gettime(ClockId::Monotonic);
    let secs = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(ts.tv_nsec).unwrap_or(0);
    secs.saturating_mul(1_000_000).saturating_add(nanos / 1_000)
}

/// Monotonic µs since the first call in this process. Comparable ONLY inside the process: the
/// input module (the one cross-process reader) is not available on Windows in 0.1.0.
#[cfg(not(unix))]
pub fn mono_us() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    let elapsed = ORIGIN.get_or_init(Instant::now).elapsed();
    u64::try_from(elapsed.as_micros()).unwrap_or(u64::MAX)
}
