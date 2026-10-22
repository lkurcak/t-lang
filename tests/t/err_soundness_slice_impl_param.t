// Nor does an impl's: its methods could have a `T` by value. So this
// impl isn't one for `&[i64]`.
trait Show {
    fn show(&self);
}

impl<T> Show for &T {
    fn show(&self) {}
}

fn main() {
    let s: &[i64] = core::slice::empty();
    Show::show(&s);
}
// error: `&[i64]` doesn't implement `Show`
