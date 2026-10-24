// Even with one impl that fits, `T` isn't worked out from `T::Out`: which
// impls there are mustn't change what a call means.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = i64;
}

fn mk<T: Tr>(x: T::Out) -> T::Out { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&mk(1i64))?;
    Result::Ok(())
}
// error: cannot fully infer
