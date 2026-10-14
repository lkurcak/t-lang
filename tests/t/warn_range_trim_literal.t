// A special case at the end of a range: just start the range after it.
fn f(n: int) -> int {
    match n {
        5 => 1,
        5..10 => 2,
        _ => 3,
    }
}
fn main() {
    print(f(5));
    print(f(6));
    print(f(10));
}
// warning: this range can be narrowed to `6..10`
// expect: 1
// expect: 2
// expect: 3
