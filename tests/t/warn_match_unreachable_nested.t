fn f(a: bool) -> int {
    match a {
        x => 1,
        true => 2,
    }
}
fn main() {
    print(f(true));
}
// warning: unreachable pattern
// expect: 1
