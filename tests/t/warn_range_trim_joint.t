// Two earlier arms cover the start between them.
fn f(n: i64) -> i64 {
    match n {
        0..10 => 1,
        10..20 => 2,
        5..30 => 3,
        _ => 4,
    }
}
fn main() {
    print(f(5));
    print(f(15));
    print(f(25));
    print(f(30));
}
// warning: this range can be narrowed to `20..30`
// expect: 1
// expect: 2
// expect: 3
// expect: 4
