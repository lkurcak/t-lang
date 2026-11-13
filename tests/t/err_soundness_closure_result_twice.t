// A closure may run more than once, giving something else each time: what
// its body made is new each time it's counted on, so two results aren't
// the same value.
fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn main() {
    let mut k: u32 = 0;
    let (a, b) = twice(|| {
        k = k / 2 + 1;
        k
    });
    let c: { v: u32 | v == a } = b;
    let _ = c;
}
// error: can't prove `v == a`
