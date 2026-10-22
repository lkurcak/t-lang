// Two ranges end to end are one range.
fn f(n: i64) -> i64 {
    match n {
        0..10 | 10..20 => 1,
        _ => 2,
    }
}
fn main() {
    print(f(-1));
    print(f(0));
    print(f(10));
    print(f(19));
    print(f(20));
}
// warning: `0..10` and `10..20` can be merged into `0..20`
// expect: 2
// expect: 1
// expect: 1
// expect: 1
// expect: 2
