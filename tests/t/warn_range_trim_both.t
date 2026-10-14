// Both ends covered: narrowed on both sides.
fn f(n: int) -> int {
    match n {
        0 => 1,
        10 => 2,
        0..=10 => 3,
        _ => 4,
    }
}
fn main() {
    print(f(0));
    print(f(5));
    print(f(10));
    print(f(11));
}
// warning: this range can be narrowed to `1..=9`
// expect: 1
// expect: 3
// expect: 2
// expect: 4
