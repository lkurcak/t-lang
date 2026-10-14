enum C { R, G }
fn f(c: C) -> int {
    match c {
        C::R | C::R => 1,
        C::G => 2,
    }
}
fn main() {}
// error: unreachable alternative
