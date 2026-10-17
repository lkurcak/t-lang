// A `T` isn't `Copy` unless a bound says so.
fn dup<T>(x: T) -> T {
    let _y = x;
    x
}
fn main() {}
// error: use of moved value `x`
