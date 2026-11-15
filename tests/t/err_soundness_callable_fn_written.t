// A function's value written over is the new one's.
fn five() -> { v: u32 | v > 0 } { 5 }

fn zero() -> u32 { 0 }

fn main() {
    let mut p = five;
    let a: { v: u32 | v > 0 } = p();
    p = zero;
    let b: { v: u32 | v > 0 } = p();
    let _ = (a, b);
}
// error: can't prove `v > 0` for `b`
