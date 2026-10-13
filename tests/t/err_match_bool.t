fn f(b: bool) -> int {
    match b { true => 1 }
}
fn main() {}
// error: non-exhaustive `match`: `false` not covered
