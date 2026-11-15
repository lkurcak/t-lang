// After an `if`, a function's value is either branch's.
fn five() -> { v: u32 | v > 0 } { 5 }

fn zero() -> u32 { 0 }

fn call<R, F: FnOnce() -> R + Discard>(f: F) -> R { f() }

fn main(os: std::Os) {
    let mut p = five;
    if os.args.len() > 3 {
        p = zero;
    }
    let x: { v: u32 | v > 0 } = call(p);
    let _ = x;
}
// error: can't prove `v > 0` for `x`
