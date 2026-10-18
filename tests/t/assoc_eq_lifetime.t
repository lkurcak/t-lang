// An equality bound that names a lifetime: where it's used, the regions of
// what it equates are related, and in a closure, the function's lifetime is
// one the closure knows.
trait Tr { type Out: Discard; fn out(&self) -> Self::Out; }
#[derive(Discard)]
struct W<'b> { r: &'b int }
impl<'b> Tr for W<'b> {
    type Out = &'b int;
    fn out(&self) -> &'b int { self.r }
}
#[derive(Discard)]
struct Hold<T: Tr> { o: T::Out }

fn field<'a, T: Tr<Out = &'a int> + Discard>(x: T) -> &'a int {
    let h = Hold::<T> { o: x.out() };
    h.o
}
fn in_closure<'a, T: Tr<Out = &'a int> + Discard>(x: T) -> int {
    let f = |y: T| -> int { *y.out() };
    f(x)
}

fn main() {
    let n = 5;
    print(*field(W { r: &n }));      // expect: 5
    print(in_closure(W { r: &n }));  // expect: 5
}
