// What a closure captured `&mut` may have changed since it was made: each
// run reads it anew, so its result says nothing of what was there.
fn twice<R, F: FnMut() -> R + Discard>(mut f: F) -> (R, R) { (f(), f()) }

fn main() {
    let mut k: u32 = 1;
    let (_, b) = twice(|| {
        let r = k;
        k = 0;
        r
    });
    let c: { v: u32 | v > 0 } = b;
    let _ = c;
}
// error: can't prove `v > 0`
