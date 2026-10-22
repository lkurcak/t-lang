// The same through a field of a projection type: `Hold<Pair<'x, 'y>>`'s
// field `v` is a `&'y i64`.
#[derive(Discard)]
struct Pair<'a, 'b> { a: &'a i64, b: &'b i64 }

trait Second {
    type Out: Discard;
}

impl<'a, 'b> Second for Pair<'a, 'b> {
    type Out = &'b i64;
}

#[derive(Discard)]
struct Hold<T: Second> { v: T::Out, t: T }

fn main() {
    let long = 1;
    let r;
    {
        let short = 2;
        let h: Hold<Pair> = Hold { v: &short, t: Pair { a: &long, b: &short } };
        r = h.v;
    }
    print(*r);
}
// error: `short` doesn't live long enough
