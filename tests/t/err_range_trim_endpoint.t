// Sharing one endpoint is almost always an off-by-one.
fn f(n: int) -> int {
    match n {
        0..=5 => 1,
        5..=10 => 2,
        _ => 3,
    }
}
fn main() {}
// error: this range can be narrowed to `6..=10`
