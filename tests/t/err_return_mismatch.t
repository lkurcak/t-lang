fn f(c: bool) -> i64 {
    if c { return true; }
    1
}
fn main() {
    print(f(true));
}
// error: expected `i64`, found `bool`
