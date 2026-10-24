// `launder` says `T::Out` is `&'a i64`, so at `T` = `W<'n>`, `'n` is `'a`:
// the result can't outlive `n`. Without that, it was a use after free.
trait Tr { type Out; fn out(&self) -> Self::Out; }
#[derive(Discard)]
struct W<'b> { r: &'b i64 }
impl<'b> Tr for W<'b> {
    type Out = &'b i64;
    fn out(&self) -> &'b i64 { self.r }
}

fn launder<'a, T: Tr<Out = &'a i64> + Discard>(x: T) -> &'a i64 { x.out() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let r: &i64;
    {
        let n = 5;
        r = launder(W { r: &n });
    }
    out.line(&(*r))?;
    Result::Ok(())
}
// error: `n` doesn't live long enough
