// A type parameter never stands for a slice, so a blanket impl for `&T`
// and one for `&[i64]` don't overlap: each type has one impl.
trait Show { fn show(&self) -> i64; }

impl<T> Show for &T {
    fn show(&self) -> i64 { 1 }
}

impl Show for &[i64] {
    fn show(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 5;
    let r = &n;
    let s: &[i64] = core::slice::empty();
    out.line(&Show::show(&r))?; // expect: 1
    out.line(&Show::show(&s)) // expect: 2
}
