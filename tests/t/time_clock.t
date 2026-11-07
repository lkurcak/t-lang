// The clock only goes forward; `sleep_until` returns once its deadline
// is past, never before; the time of day is after 2023. `Clock` is
// `Copy`, so threads each take one.
use core::time::Duration;
use std::io::Error;
use std::time::{Clock, Instant};

fn waited(clock: Clock, ms: u64) -> bool {
    let start = clock.now();
    let deadline = start.add(Duration::from_millis(if ms <= 1000 { ms } else { 1000 }));
    clock.sleep_until(deadline);
    clock.now() >= deadline
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let clock = os.clock;
    let a = clock.now();
    let b = clock.now();
    out.line(&(b >= a))?;                                                  // expect: true
    out.line(&b.checked_since(a).is_some())?;                              // expect: true
    out.line(&a.since(b.add(Duration::from_secs(1))).is_zero())?;          // expect: true
    out.line(&a.checked_since(a.add(Duration::from_nanos(1))).is_none())?; // expect: true
    out.line(&waited(clock, 15))?;                                         // expect: true
    // A deadline already past doesn't wait.
    clock.sleep_until(a);
    out.line(&(clock.now().since(a) >= Duration::from_millis(15)))?; // expect: true
    let late = Instant::add(&a, Duration::max());
    out.line(&a.checked_add(Duration::max()).is_none())?; // expect: true
    out.line(&(late > a))?;                               // expect: true
    let t = os.threads.spawn(|| waited(clock, 5));
    let here = waited(clock, 5);
    let there = t.join();
    out.line(&here)?;  // expect: true
    out.line(&there)?; // expect: true
    match clock.unix_time() {
        Some(d) => out.line(&(d.as_secs() > 1_700_000_000))?, // expect: true
        None => out.line(&"before 1970")?,
    }
    Ok(())
}
