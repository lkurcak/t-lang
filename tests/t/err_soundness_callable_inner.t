// A closure that a closure's call gives back is a value of that run's:
// two calls give two closures, whatever their bodies say of what they
// captured (docs/78).
fn main() {
    let mk = |k: u32| move || k;
    let f = mk(0);
    let g = mk(5);
    if f() == 0 {
        let x: { v: u32 | v == 0 } = g();
        let _ = x;
    }
}
// error: can't prove `v == 0` for `x`
