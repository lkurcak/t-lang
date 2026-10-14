// Covered by the arm above, alternative by alternative.
enum C { R, G, B }
fn f(c: C) -> int {
    match c {
        C::R | C::G => 1,
        C::B => 2,
        C::G | C::B => 3,
    }
}
fn main() {
    print(f(C::R));
    print(f(C::G));
    print(f(C::B));
}
// warning: unreachable pattern
// expect: 1
// expect: 1
// expect: 2
