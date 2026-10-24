// An equality bound on a struct holds wherever the struct is used.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = bool;
}

#[derive(Discard)]
struct S<T: Tr<Out = i64>> { x: T::Out }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s: S<bool> = S { x: 1 };
    out.line(&s.x)?;
    Result::Ok(())
}
// error: `<bool as Tr>::Out` is `bool`, but `S` needs it to be `i64`
