// A whole run is merged at once, whatever order it is written in. The
// result takes the style of the highest range.
fn f(n: int) -> int {
    match n {
        10..=14 | 0..5 | 5..10 => 1,
        _ => 2,
    }
}
fn main() {}
// error: `0..5`, `5..10` and `10..=14` can be merged into `0..=14`
