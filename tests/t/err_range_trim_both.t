// Both ends covered: narrowed on both sides.
fn f(n: int) -> int {
    match n {
        0 => 1,
        10 => 2,
        0..=10 => 3,
        _ => 4,
    }
}
fn main() {}
// error: this range can be narrowed to `1..=9`
