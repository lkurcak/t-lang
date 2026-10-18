#[derive(Discard)]
enum C { R, G }
fn f(c: C) -> int {
    match c {
        C::R | C::R => 1,
        C::G => 2,
    }
}
fn main() {
    print(f(C::R));
    print(f(C::G));
}
// warning: unreachable alternative
// expect: 1
// expect: 2
