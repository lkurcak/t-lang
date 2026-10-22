fn f(c: bool) -> i64 {
    if c { return; }
    1
}
fn main() {
    print(f(true));
}
// error: a `return` with no value returns `()`, but this returns `i64`
