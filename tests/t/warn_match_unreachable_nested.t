fn f(a: bool) -> i64 {
    match a {
        _x => 1,
        true => 2,
    }
}
fn main() {
    print(f(true));
}
// warning: unreachable pattern
// expect: 1
