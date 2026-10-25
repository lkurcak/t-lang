// The logic has integers and `bool`s: a measure gives one.
#[derive(Copy, Discard)]
struct W<'a> { s: &'a [i64] }

impl<'a> W<'a> {
    #[measure]
    fn all(&self) -> &'a [i64] { self.s }
}

fn main() {}
// error: a measure gives an integer or a `bool`, not `&'a [i64]`
