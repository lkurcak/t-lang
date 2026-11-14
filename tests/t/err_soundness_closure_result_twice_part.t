// Two runs give two values, enums' too: what a run made of an enum's
// value, its variant and fields, is new each time it's counted on.
fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn main() with panic {
    let mut k: u32 = 0;
    let (a, b) = twice(|| {
        k = k / 2 + 1;
        Some(k)
    });
    let x = a.unwrap();
    let y = b.unwrap();
    let c: { v: u32 | v == x } = y;
    let _ = c;
}
// error: can't prove `v == x` for `c`
