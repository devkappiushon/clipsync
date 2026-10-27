use rustix::time::{ClockId, clock_gettime};

/// CLOCK_MONOTONIC in µs. Comparable across processes ONLY on the same machine (loopback).
pub fn mono_us() -> u64 {
    let ts = clock_gettime(ClockId::Monotonic);
    let secs = u64::try_from(ts.tv_sec).unwrap_or(0);
    let nanos = u64::try_from(ts.tv_nsec).unwrap_or(0);
    secs.saturating_mul(1_000_000).saturating_add(nanos / 1_000)
}
