fn apply(f: fn(i64) -> i64) -> i64 { f(1) }
fn main() {
    print(apply(|x: bool| 1));
}
// error: expected `i64`, found `bool`
