// An equality bound that names a lifetime: where it's used, the regions of
// what it equates are related, and in a closure, the function's lifetime is
// one the closure knows.
trait Tr { type Out: Discard; fn out(&self) -> Self::Out; }
#[derive(Discard)]
struct W<'b> { r: &'b i64 }
impl<'b> Tr for W<'b> {
    type Out = &'b i64;
    fn out(&self) -> &'b i64 { self.r }
}
#[derive(Discard)]
struct Hold<T: Tr> { o: T::Out }

fn field<'a, T: Tr<Out = &'a i64> + Discard>(x: T) -> &'a i64 {
    let h = Hold::<T> { o: x.out() };
    h.o
}
fn in_closure<'a, T: Tr<Out = &'a i64> + Discard>(x: T) -> i64 {
    let f = |y: T| -> i64 { *y.out() };
    f(x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n = 5;
    out.line(&(*field(W { r: &n })))?;      // expect: 5
    out.line(&in_closure(W { r: &n }))?;  // expect: 5
    Result::Ok(())
}
