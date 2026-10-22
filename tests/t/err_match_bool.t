fn f(b: bool) -> i64 {
    match b { true => 1 }
}
fn main() {}
// error: non-exhaustive `match`: `false` not covered
