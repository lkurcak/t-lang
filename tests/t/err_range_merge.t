// Two ranges end to end are one range.
fn f(n: int) -> int {
    match n {
        0..10 | 10..20 => 1,
        _ => 2,
    }
}
fn main() {}
// error: `0..10` and `10..20` can be merged into `0..20`
