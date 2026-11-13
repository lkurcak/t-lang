// A sealed trait's blanket impl on an open trait of the program's own rules
// out what the program doesn't make `Open`: only it could, so `char` isn't
// `Small`, and has a `Describe` of its own.
trait Open {}
sealed trait Small {}

impl<T: Open> Small for T {}

impl Open for bool {}

trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Small> Describe for T {
    fn describe(&self) -> i64 { 1 }
}

impl Describe for char {
    fn describe(&self) -> i64 { 2 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&'c'.describe())?; // expect: 2
    out.line(&true.describe()) // expect: 1
}
