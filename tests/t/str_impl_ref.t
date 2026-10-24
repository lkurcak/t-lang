// A type parameter never stands for `str`, so a blanket impl for `&T`
// doesn't cover `&str`, and one for `&str` doesn't overlap it.
trait Show { fn show(&self) -> i64; }

impl<T> Show for &T {
    fn show(&self) -> i64 { 1 }
}

impl Show for &str {
    fn show(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let n: i64 = 5;
    let r = &n;
    let s = "five";
    out.line(&Show::show(&r))?; // expect: 1
    out.line(&Show::show(&s)) // expect: 2
}
