// A single int at the end of a range counts too.
fn f(n: int) -> int {
    match n {
        0..10 | 10 => 1,
        _ => 2,
    }
}
fn main() {
    print(f(9));
    print(f(10));
    print(f(11));
}
// warning: `0..10` and `10` can be merged into `0..=10`
// expect: 1
// expect: 1
// expect: 2
