// An impl's method's parameters are its trait's, in order: a const
// parameter where the trait's has a type parameter is refused as that, not
// as the types it makes.
trait K {
    fn f<const N: usize, T: Discard>(a: [T; N]) -> usize;
}

#[derive(Discard)]
struct R {}

impl K for R {
    fn f<T: Discard, const N: usize>(a: [T; N]) -> usize { N }
}

fn main() {}
// error: its `T` is a type parameter, where the trait's `N` is a const parameter
