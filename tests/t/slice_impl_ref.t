// A type parameter never stands for a slice, so a blanket impl for `&T`
// and one for `&[int]` don't overlap: each type has one impl.
trait Show { fn show(&self); }

impl<T> Show for &T {
    fn show(&self) { print(1); }
}

impl Show for &[int] {
    fn show(&self) { print(2); }
}

fn main() {
    let n = 5;
    let r = &n;
    let s: &[int] = core::slice::empty();
    Show::show(&r); // expect: 1
    Show::show(&s); // expect: 2
}
