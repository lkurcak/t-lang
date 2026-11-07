// `Duration` is a whole number of nanoseconds: made from a unit whose
// limit the checker proves, read back in any unit, added and taken away
// without ever panicking.
use core::time::{Duration, MAX_SECS};
use std::io::Error;

fn secs(n: u64) -> Option<Duration> {
    if n <= MAX_SECS { Some(Duration::from_secs(n)) } else { None }
}

fn main(os: std::Os) -> Result<(), Error> {
    let mut out = os.stdout;
    let d = Duration::from_millis(1_500);
    out.line(&d.as_secs())?;                                                    // expect: 1
    out.line(&d.as_millis())?;                                                  // expect: 1500
    out.line(&d.as_micros())?;                                                  // expect: 1500000
    out.line(&d.subsec_nanos())?;                                               // expect: 500000000
    out.line(&(d == Duration::from_micros(1_500_000)))?;                        // expect: true
    out.line(&(d < Duration::from_secs(2)))?;                                   // expect: true
    out.line(&(Duration::from_nanos(7) > Duration::zero()))?;                   // expect: true
    out.line(&d.saturating_sub(Duration::from_secs(2)).is_zero())?;             // expect: true
    out.line(&d.checked_sub(Duration::from_secs(2)).is_none())?;                // expect: true
    out.line(&Duration::max().checked_add(Duration::from_nanos(1)).is_none())?; // expect: true
    out.line(&(Duration::max().saturating_add(d) == Duration::max()))?;         // expect: true
    out.line(&Duration::from_secs(MAX_SECS).as_secs())?; // expect: 18446744073
    out.line(&secs(MAX_SECS + 1).is_none())?;            // expect: true
    out.line(&f"{Duration::from_nanos(3):?}")?;          // expect: Duration { nanos: 3 }
    Ok(())
}
