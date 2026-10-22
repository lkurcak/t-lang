// Every warning is reported, not just the first, in source order.
fn f(n: i64) -> i64 {
    match n {
        0..=5 => 1,
        5..=10 => 2,
        3 => 3,
        _ => 4,
    }
}
fn g(b: bool) -> i64 {
    match b {
        true | true => 1,
        false => 2,
    }
}
fn main() {
    print(f(5));
    print(f(6));
    print(g(true));
}
// warning: this range can be narrowed to `6..=10`
// warning: unreachable pattern
// warning: unreachable alternative
// expect: 1
// expect: 2
// expect: 1
