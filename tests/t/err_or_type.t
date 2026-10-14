enum E { A(int), B(bool) }
fn f(e: E) -> int {
    match e {
        E::A(x) | E::B(x) => 0,
    }
}
fn main() {}
// error: `x` is `int` in the first alternative but `bool` in this one
