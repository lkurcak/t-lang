fn f(c: bool) -> int {
    if c { return; }
    1
}
fn main() {
    print(f(true));
}
// error: a `return` with no value returns `()`, but this returns `int`
