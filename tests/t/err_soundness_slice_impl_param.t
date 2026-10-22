// Nor does an impl's: its methods could have a `T` by value. So this
// impl isn't one for `&[int]`.
trait Show {
    fn show(&self);
}

impl<T> Show for &T {
    fn show(&self) {}
}

fn main() {
    let s: &[int] = core::slice::empty();
    Show::show(&s);
}
// error: `&[int]` doesn't implement `Show`
