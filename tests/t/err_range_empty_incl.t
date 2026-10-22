fn f(n: i64) -> i64 {
    match n { 3..=2 => 1, _ => 0 }
}
fn main() {}
// error: this range is empty
