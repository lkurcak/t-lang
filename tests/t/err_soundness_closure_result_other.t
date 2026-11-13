// What a call gives back of `R` may be the closure's, or an argument's:
// both must have it.
fn pick<R: Discard, F: FnOnce() -> R + Discard>(r: R, c: bool, f: F) -> R {
    match c {
        true => f(),
        false => r,
    }
}

fn five() -> { v: u32 | v > 0 } { 5 }

fn main() {
    let z: u32 = 0;
    let c: { v: u32 | v > 0 } = pick(z, false, || five());
    let _ = c;
}
// error: can't prove `v > 0`
