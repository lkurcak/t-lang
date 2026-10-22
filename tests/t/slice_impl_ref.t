// A type parameter never stands for a slice, so a blanket impl for `&T`
// and one for `&[i64]` don't overlap: each type has one impl.
trait Show { fn show(&self); }

impl<T> Show for &T {
    fn show(&self) { print(1i64); }
}

impl Show for &[i64] {
    fn show(&self) { print(2i64); }
}

fn main() {
    let n: i64 = 5;
    let r = &n;
    let s: &[i64] = core::slice::empty();
    Show::show(&r); // expect: 1
    Show::show(&s); // expect: 2
}
