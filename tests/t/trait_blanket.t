// A blanket impl, for every type that implements another trait. Proving
// `i64: Describe` needs `i64: Show`, for the same type: fine, as long as
// nothing leads back from `Show` to `Describe`.

trait Show {
    fn show(&self) -> i64;
}

trait Describe {
    fn describe(&self) -> i64;
}

impl<T: Show> Describe for T {
    fn describe(&self) -> i64 { self.show() }
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&5i64.describe()) // expect: 5
}
