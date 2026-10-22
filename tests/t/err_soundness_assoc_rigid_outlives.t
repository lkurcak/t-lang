// In generic code, `T::Out` is made of `T`'s lifetimes and `'static`, so
// it lives as long as `T` does, and maybe no longer: a `Hold<'r, T::Out>`,
// which needs `T::Out: 'r`, needs `T: 'r`, which nothing says. At `T =
// &'x i64` it would need `'x: 'r`.
trait Tr {
    type Out: Discard;
}

#[derive(Discard)]
struct Hold<'r, U: 'r> { u: U, r: &'r i64 }

fn g<'r, T: Tr>(x: T::Out, r: &'r i64) -> i64 {
    let h: Hold<'r, T::Out> = Hold { u: x, r };
    *h.r
}

fn main() {}
// error: the type `T` may not live long enough: this needs it to outlive `'r`; say it does with a bound, `T: 'r`
