fn f(n: int) -> int {
    match n {
        0..=9 | 3 => 1,
        _ => 2,
    }
}
fn main() {}
// error: unreachable
