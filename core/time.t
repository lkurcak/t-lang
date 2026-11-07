//! Spans of time: `Duration`. Reading a clock needs one, which only an
//! operating system has, so clocks are std's (`std::time`); a span of time
//! is just a number, so it's here, for code with no clock too.

use core::cmp::{Eq, Ord, Ordering};
use core::num::{checked_add, checked_sub};

/// A span of time, never negative: a whole number of nanoseconds, up to
/// `u64`'s greatest, which is more than 584 years.
#[derive(Discard, Copy, Debug, Send, Sync)]
export struct Duration { nanos: u64 }

/// The most seconds a `Duration` holds: `Duration::from_secs` takes no
/// more.
export const MAX_SECS: u64 = 18_446_744_073;

/// The most milliseconds a `Duration` holds.
export const MAX_MILLIS: u64 = 18_446_744_073_709;

/// The most microseconds a `Duration` holds.
export const MAX_MICROS: u64 = 18_446_744_073_709_551;

impl Duration {
    /// No time at all.
    export fn zero() -> Duration { Duration { nanos: 0 } }

    /// The longest `Duration` there is.
    export fn max() -> Duration { Duration { nanos: 18_446_744_073_709_551_615 } }

    /// `n` nanoseconds.
    export fn from_nanos(n: u64) -> Duration { Duration { nanos: n } }

    /// `n` microseconds, of which there are at most `MAX_MICROS`.
    export fn from_micros(n: { n: u64 | n <= MAX_MICROS }) -> Duration {
        Duration { nanos: n * 1_000 }
    }

    /// `n` milliseconds, of which there are at most `MAX_MILLIS`.
    export fn from_millis(n: { n: u64 | n <= MAX_MILLIS }) -> Duration {
        Duration { nanos: n * 1_000_000 }
    }

    /// `n` seconds, of which there are at most `MAX_SECS`.
    export fn from_secs(n: { n: u64 | n <= MAX_SECS }) -> Duration {
        Duration { nanos: n * 1_000_000_000 }
    }

    /// How many whole nanoseconds it is: all of it.
    export fn as_nanos(&self) -> u64 { self.nanos }

    /// How many whole microseconds it is, leaving out what's less.
    export fn as_micros(&self) -> u64 { self.nanos / 1_000 }

    /// How many whole milliseconds it is, leaving out what's less.
    export fn as_millis(&self) -> u64 { self.nanos / 1_000_000 }

    /// How many whole seconds it is, leaving out what's less.
    export fn as_secs(&self) -> u64 { self.nanos / 1_000_000_000 }

    /// The nanoseconds past its whole seconds: less than a billion.
    export fn subsec_nanos(&self) -> u32 { core::num::wrap(self.nanos % 1_000_000_000) }

    /// Is it no time at all?
    export fn is_zero(&self) -> bool { self.nanos == 0 }

    /// The two together, or `None` if that's longer than any `Duration`.
    export fn checked_add(&self, other: Duration) -> Option<Duration> {
        match checked_add(self.nanos, other.nanos) {
            Some(n) => Some(Duration { nanos: n }),
            None => None,
        }
    }

    /// `other` taken from this, or `None` if it's longer: a `Duration`
    /// isn't negative.
    export fn checked_sub(&self, other: Duration) -> Option<Duration> {
        match checked_sub(self.nanos, other.nanos) {
            Some(n) => Some(Duration { nanos: n }),
            None => None,
        }
    }

    /// The two together, or the longest `Duration` if that's longer.
    export fn saturating_add(&self, other: Duration) -> Duration {
        match self.checked_add(other) {
            Some(d) => d,
            None => Duration::max(),
        }
    }

    /// `other` taken from this, or no time if it's longer.
    export fn saturating_sub(&self, other: Duration) -> Duration {
        match self.checked_sub(other) {
            Some(d) => d,
            None => Duration::zero(),
        }
    }
}

/// Two spans are equal when they're as long.
impl Eq for Duration {
    fn eq(&self, other: &Duration) -> bool { self.nanos == other.nanos }
}

/// The shorter span is the lesser.
impl Ord for Duration {
    fn cmp(&self, other: &Duration) -> Ordering { self.nanos.cmp(&other.nanos) }
}
