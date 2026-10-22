fn apply(f: fn(i64) -> i64) -> i64 { f(1) }
fn main() {
    print(apply(|x, y| x + y));
}
// error: expected `fn(i64) -> i64`, found `fn(_, _) -> _`
