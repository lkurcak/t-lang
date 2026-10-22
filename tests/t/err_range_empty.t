fn f(n: i64) -> i64 {
    match n { 5..5 => 1, _ => 0 }
}
fn main() {}
// error: this range is empty, so it can never match
