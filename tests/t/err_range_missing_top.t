// `..10` stops short of 10.
fn f(n: i64) -> i64 {
    match n { ..10 => 1, 11.. => 2 }
}
fn main() {}
// error: non-exhaustive `match`: `10` not covered
