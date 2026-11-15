// A function's value lent `&mut` may be another function after.
fn five() -> { v: u32 | v > 0 } { 5 }

fn zero() -> u32 { 0 }

fn set<T: Discard>(a: &mut T, b: T) {
    *a = b;
}

fn main() {
    let mut p = five;
    set(&mut p, zero);
    let x: { v: u32 | v > 0 } = p();
    let _ = x;
}
// error: can't prove `v > 0` for `x`
