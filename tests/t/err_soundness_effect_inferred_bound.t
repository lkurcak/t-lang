// A closure that calls an inferred function has its effects, so it can't
// meet a bound that allows none.
fn g() -> i64 { panic("boom") }

fn twice<F: Fn() -> i64 + Discard>(f: F) -> i64 { core::num::wrapping_add(f(), f()) }

fn main() with panic { let _ = twice(|| g()); }
// error: this closure may panic, but `twice` needs calls of it to fit
