// A closure's parameters are only in scope in its body.
fn main() {
    let f = |x: int| x;
    print(f(1) + x);
}
// error: unknown variable `x`
