// `W<'m, 'b>` gets shorter with `'b`, but `<W<'m, 'b> as Tr>::Out`, a `&'m
// mut &'b i64`, can't: so `H<T>`, holding a `T::Out`, mustn't either. If
// `H` were covariant in `T`, `shrink` would let a short-lived reference
// into a slot that outlives it.
#[derive(Discard)]
struct W<'m, 'b> { m: &'m i64, b: &'b i64 }

trait Tr {
    type Out: Discard;
}

impl<'m, 'b> Tr for W<'m, 'b> {
    type Out = &'m mut &'b i64;
}

#[derive(Discard)]
struct H<T: Tr> { f: T::Out }

fn shrink<'m, 's, 'l: 's>(h: H<W<'m, 'l>>) -> H<W<'m, 's>> { h }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let long = 1;
    let mut slot = &long;
    {
        let short = 2;
        let h: H<W> = H { f: &mut slot };
        let h = shrink(h);
        *h.f = &short;
    }
    out.line(&(*slot))?;
    Result::Ok(())
}
// error: lifetime may not live long enough: this needs `'s` to outlive `'l`
