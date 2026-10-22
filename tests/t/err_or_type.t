#[derive(Discard)]
enum E { A(i64), B(bool) }
fn f(e: E) -> i64 {
    match e {
        E::A(x) | E::B(x) => 0,
    }
}
fn main() {}
// error: `x` is `i64` in the first alternative but `bool` in this one
