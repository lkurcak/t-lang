fn f(n: int) -> int {
    match n { 3..=2 => 1, _ => 0 }
}
fn main() {}
// error: this range is empty
