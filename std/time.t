//! Time: the program's `Clock`, which is in the `Os` that `main` is
//! given, as `os.clock`, and the `Instant`s it tells (see
//! docs/49-time-entropy-terminal.md). A span of time is a
//! `core::time::Duration`.
//!
//! Reading a clock gives a different answer each time, so a function can
//! read one only if it was handed it: one handed no `Clock` does the same
//! whenever it's called.

use core::cmp::{Eq, Ord, Ordering};
use core::time::Duration;

/// The program's clock: what `fn main(os: Os)` is given in its
/// `os.clock`. It only tells the time, so it's `Copy`: anything may have
/// one, and threads too.
#[derive(Discard, Copy, Send, Sync)]
#[intrinsic]
export struct Clock {}

/// A moment, as the `Clock` told it: `clock.now()`. Instants only ever
/// grow, as the program runs, whatever the time of day is set to, so how
/// far apart two are is how long passed between them. What one is on its
/// own means nothing: it isn't a time of day.
#[derive(Discard, Copy, Debug, Send, Sync)]
export struct Instant { pub nanos: u64 }

impl Clock {
    /// Now. No `Instant` it told before is later.
    export fn now(&self) -> Instant { Instant { nanos: self.os_now() } }

    /// Waits until `deadline` is past: until `now()` is no earlier. If it
    /// is already, it doesn't wait.
    ///
    /// There's no waiting for a while, only until a moment: a loop that
    /// does something every second, and waited a second after each time,
    /// would drift by however long it took. One that keeps the moment it
    /// should next run, and adds a second to it, doesn't.
    export fn sleep_until(&self, deadline: Instant) { self.os_sleep_until(deadline.nanos) }

    /// How long it's been since 1970 began, in UTC, by the time of day the
    /// operating system has: `None` only if it says that's still to come.
    /// The time of day can be set, back too, so unlike `now()`, what this
    /// says can go down.
    export fn unix_time(&self) -> Option<Duration> {
        let n = self.os_unix_time();
        if n == 18_446_744_073_709_551_615 { None } else { Some(Duration::from_nanos(n)) }
    }

    /// Now, in nanoseconds, from some moment before the program began.
    #[intrinsic]
    fn os_now(&self) -> u64;

    /// Waits until `os_now()` is no less than `deadline`.
    #[intrinsic]
    fn os_sleep_until(&self, deadline: u64);

    /// The nanoseconds since 1970, or `u64`'s greatest if that's before.
    #[intrinsic]
    fn os_unix_time(&self) -> u64;
}

impl Instant {
    /// How long after `earlier` this is: no time, if it isn't after it.
    export fn since(&self, earlier: Instant) -> Duration {
        Duration::from_nanos(self.nanos).saturating_sub(Duration::from_nanos(earlier.nanos))
    }

    /// How long after `earlier` this is, or `None` if it's before it.
    export fn checked_since(&self, earlier: Instant) -> Option<Duration> {
        Duration::from_nanos(self.nanos).checked_sub(Duration::from_nanos(earlier.nanos))
    }

    /// The moment `d` after this one, or the last there is, if that's
    /// later: a deadline that's never reached.
    export fn add(&self, d: Duration) -> Instant {
        Instant { nanos: Duration::from_nanos(self.nanos).saturating_add(d).as_nanos() }
    }

    /// The moment `d` after this one, or `None` if that's later than any.
    export fn checked_add(&self, d: Duration) -> Option<Instant> {
        match Duration::from_nanos(self.nanos).checked_add(d) {
            Some(t) => Some(Instant { nanos: t.as_nanos() }),
            None => None,
        }
    }
}

/// Two instants are equal when they're the same moment.
impl Eq for Instant {
    fn eq(&self, other: &Instant) -> bool { self.nanos == other.nanos }
}

/// The earlier instant is the lesser.
impl Ord for Instant {
    fn cmp(&self, other: &Instant) -> Ordering { self.nanos.cmp(&other.nanos) }
}
