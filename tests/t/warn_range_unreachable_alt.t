fn f(n: i64) -> i64 {
    match n {
        0..=9 | 3 => 1,
        _ => 2,
    }
}
fn main() {
    print(f(3));
    print(f(10));
}
// warning: unreachable
// expect: 1
// expect: 2
