// A whole run is merged at once, whatever order it is written in. The
// result takes the style of the highest range.
fn f(n: i64) -> i64 {
    match n {
        10..=14 | 0..5 | 5..10 => 1,
        _ => 2,
    }
}
fn main() {
    print(f(0));
    print(f(7));
    print(f(14));
    print(f(15));
}
// warning: `0..5`, `5..10` and `10..=14` can be merged into `0..=14`
// expect: 1
// expect: 1
// expect: 1
// expect: 2
