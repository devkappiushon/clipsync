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

/// Monotonic µs since an `Instant` fixed on the first call in this process. Comparable ONLY inside the
/// process: the input module (the one cross-process reader) is not available on Windows in 0.1.0.
#[cfg(not(unix))]
pub fn mono_us() -> u64 {
    use std::sync::OnceLock;
    use std::time::Instant;
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    micros_since(*ORIGIN.get_or_init(Instant::now))
}

/// The origin counts as 1 µs before the first call, so the value is never 0, as on Unix (where 0 would
/// only come from a broken clock).
#[cfg(any(not(unix), test))]
fn micros_since(origin: std::time::Instant) -> u64 {
    u64::try_from(origin.elapsed().as_micros())
        .unwrap_or(u64::MAX)
        .saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::micros_since;
    use std::time::{Duration, Instant};

    #[test]
    fn per_process_clock_never_returns_zero_and_advances() {
        let origin = Instant::now();
        let a = micros_since(origin);
        assert!(a >= 1, "{a}");
        std::thread::sleep(Duration::from_millis(2));
        assert!(micros_since(origin) >= a + 2_000);
    }
}
