// An impl with an equality bound is only for the types that meet it.
trait Tr {
    type Out;
}

#[derive(Discard)]
struct B { b: bool }

impl Tr for B {
    type Out = bool;
}

trait Show {
    fn show(&self) -> i64;
}

#[derive(Discard)]
struct W<T> { t: T }

impl<T: Tr<Out = i64>> Show for W<T> {
    fn show(&self) -> i64 { 1 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(W { t: B { b: true } }).show())?;
    Result::Ok(())
}
// error: `W<B>` doesn't implement `Show`
