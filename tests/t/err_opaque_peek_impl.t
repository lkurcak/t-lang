// `i64` implements `Tr`, but callers don't know `f` returns one.
trait Show {
    fn show(&self) -> i64;
}

impl Show for i64 {
    fn show(&self) -> i64 { *self }
}

trait Tr {
    fn t(&self) -> i64;
}

impl Tr for i64 {
    fn t(&self) -> i64 { 1 }
}

fn f() -> impl Show + Discard { 1i64 }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&f().t())?;  Result::Ok(()) }
// error: `impl Show + Discard` has no method `t`; trait `Tr` has one, but `impl Show + Discard` doesn't implement it
