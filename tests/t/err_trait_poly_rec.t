// Each call is at a bigger type than the one before: `W<int>`,
// `W<&W<int>>`, `W<&W<&W<int>>>`...
trait Tr { fn f(&self) -> int; }
struct W<X> { x: X }
impl<X: Tr> Tr for W<X> {
    fn f(&self) -> int { W { x: self }.f() }
}
impl<'a, X: Tr> Tr for &'a X {
    fn f(&self) -> int { (**self).f() }
}
impl Tr for int { fn f(&self) -> int { *self } }
fn main() { print(W { x: 1 }.f()); }
// error: polymorphic recursion: `<W<X> as Tr>::f` uses itself with `X` = `&W<X>`
