// A type parameter never stands for `str`, so a blanket impl for `&T`
// doesn't cover `&str`, and one for `&str` doesn't overlap it.
trait Show { fn show(&self); }

impl<T> Show for &T {
    fn show(&self) { print(1i64); }
}

impl Show for &str {
    fn show(&self) { print(2i64); }
}

fn main() {
    let n: i64 = 5;
    let r = &n;
    let s = "five";
    Show::show(&r); // expect: 1
    Show::show(&s); // expect: 2
}
