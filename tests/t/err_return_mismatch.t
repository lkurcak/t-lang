fn f(c: bool) -> int {
    if c { return true; }
    1
}
fn main() {
    print(f(true));
}
// error: expected `int`, found `bool`
