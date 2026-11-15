// A parameter the body changes: what the call's argument was is what it
// was first.
fn main() {
    let f = |mut x: u32| {
        x = x / 2;
        x
    };
    let a: { v: u32 | v <= 2 } = f(4);
    let b: { v: u32 | v == 4 } = f(4);
    let _ = (a, b);
}
// error: can't prove `v == 4` for `b`
