// Earlier alternatives count, as for unreachable alternatives.
fn f(n: int) -> int {
    match n {
        0..10 | 5..20 => 1,
        _ => 2,
    }
}
fn main() {
    print(f(3));
    print(f(15));
    print(f(20));
}
// warning: this range can be narrowed to `10..20`
// expect: 1
// expect: 1
// expect: 2
