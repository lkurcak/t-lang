// Two ranges between them cover the third, though neither does alone.
fn f(n: int) -> int {
    match n {
        0..10 => 1,
        10..=20 => 2,
        5..=15 => 3,
        _ => 4,
    }
}
fn main() {}
// error: unreachable pattern
