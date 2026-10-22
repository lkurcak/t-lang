// An impl for `&'a i64` is for every `'a`, so its method can't make one out
// of a shorter reference.
trait Mk {
    fn mk(x: &i64) -> Self;
}

impl<'a> Mk for &'a i64 {
    fn mk(x: &i64) -> &'a i64 { x }
}

fn main() {}
// error: lifetime may not live long enough
