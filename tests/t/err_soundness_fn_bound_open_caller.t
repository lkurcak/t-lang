// What an open bound's calls do is decided where it's called: here, by a
// closure that may panic, which `quiet` says it can't.
fn twice<F: Fn() -> i64 + Discard>(f: F) -> i64 { core::num::wrapping_add(f(), f()) }

fn quiet(z: i64) -> i64 with {} { twice(|| 10 / z) }

fn main() {
    let _ = quiet(0);
}
// error: `twice` may panic, but `quiet` doesn't say `with panic`
