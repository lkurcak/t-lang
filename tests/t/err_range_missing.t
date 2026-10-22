// The gap between the ranges is reported as a range.
fn f(n: i64) -> i64 {
    match n { ..0 => 1, 10.. => 2, 0..5 => 3 }
}
fn main() {}
// error: non-exhaustive `match`: `5..=9` not covered
