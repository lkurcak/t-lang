// `W<'m, 'b>` gets shorter with `'b`, but `<W<'m, 'b> as Tr>::Out`, a `&'m
// mut &'b int`, can't: so `H<T>`, holding a `T::Out`, mustn't either. If
// `H` were covariant in `T`, `shrink` would let a short-lived reference
// into a slot that outlives it.
#[derive(Discard)]
struct W<'m, 'b> { m: &'m int, b: &'b int }

trait Tr {
    type Out: Discard;
}

impl<'m, 'b> Tr for W<'m, 'b> {
    type Out = &'m mut &'b int;
}

#[derive(Discard)]
struct H<T: Tr> { f: T::Out }

fn shrink<'m, 's, 'l: 's>(h: H<W<'m, 'l>>) -> H<W<'m, 's>> { h }

fn main() {
    let long = 1;
    let mut slot = &long;
    {
        let short = 2;
        let h: H<W> = H { f: &mut slot };
        let h = shrink(h);
        *h.f = &short;
    }
    print(*slot);
}
// error: lifetime may not live long enough: this needs `'s` to outlive `'l`
