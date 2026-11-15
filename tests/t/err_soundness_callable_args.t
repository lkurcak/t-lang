// A closure's parameters are the arguments of the call it's called with,
// and no others.
fn main() {
    let f = |x: usize| x;
    let a: { v: usize | v == 3 } = f(3);
    let b: { v: usize | v == 3 } = f(9);
    let _ = (a, b);
}
// error: can't prove `v == 3` for `b`
