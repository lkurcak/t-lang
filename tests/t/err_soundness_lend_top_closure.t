// A closure given a `&mut` to a refined place is walked with what's
// declared there: what it writes through it must have it.
fn with_mut<T, F: FnOnce(&mut T) + Discard>(a: &mut T, f: F) { f(a) }

fn main() {
    let mut a: { v: u32 | v > 0 } = 1;
    with_mut(&mut a, |x| *x = 0);
    let _ = a;
}
// error: can't prove `v > 0` for what's written through `*x`
