fn f(a: bool) -> int {
    match a {
        x => 1,
        true => 2,
    }
}
fn main() {}
// error: unreachable pattern
