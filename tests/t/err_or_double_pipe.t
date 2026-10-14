enum C { R, G }
fn f(c: C) -> int {
    match c { C::R || C::G => 1 }
}
fn main() {}
// error: use `|` between pattern alternatives, not `||`
