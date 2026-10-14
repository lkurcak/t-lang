fn f(n: int) -> int {
    match n { 5..5 => 1, _ => 0 }
}
fn main() {}
// error: this range is empty, so it can never match
