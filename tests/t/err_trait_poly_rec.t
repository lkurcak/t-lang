// Each call is at a bigger type than the one before: `W<i64>`,
// `W<&W<i64>>`, `W<&W<&W<i64>>>`...
trait Tr { fn f(&self) -> i64; }
#[derive(Discard)]
struct W<X> { x: X }
impl<X: Tr> Tr for W<X> {
    fn f(&self) -> i64 { W { x: self }.f() }
}
impl<'a, X: Tr> Tr for &'a X {
    fn f(&self) -> i64 { (**self).f() }
}
impl Tr for i64 { fn f(&self) -> i64 { *self } }
fn main() { print(W { x: 1i64 }.f()); }
// error: polymorphic recursion: `<W<X> as Tr>::f` uses itself with `X` = `&W<X>`
