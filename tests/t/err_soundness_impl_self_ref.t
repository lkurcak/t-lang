// An impl for `&'a int` is for every `'a`, so its method can't make one out
// of a shorter reference.
trait Mk {
    fn mk(x: &int) -> Self;
}

impl<'a> Mk for &'a int {
    fn mk(x: &int) -> &'a int { x }
}

fn main() {}
// error: lifetime may not live long enough
