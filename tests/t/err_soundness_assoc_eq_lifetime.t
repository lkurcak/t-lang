// `launder` says `T::Out` is `&'a int`, so at `T` = `W<'n>`, `'n` is `'a`:
// the result can't outlive `n`. Without that, it was a use after free.
trait Tr { type Out; fn out(&self) -> Self::Out; }
#[derive(Discard)]
struct W<'b> { r: &'b int }
impl<'b> Tr for W<'b> {
    type Out = &'b int;
    fn out(&self) -> &'b int { self.r }
}

fn launder<'a, T: Tr<Out = &'a int> + Discard>(x: T) -> &'a int { x.out() }

fn main() {
    let r: &int;
    {
        let n = 5;
        r = launder(W { r: &n });
    }
    print(*r);
}
// error: `n` doesn't live long enough
