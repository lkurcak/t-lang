// `mk`'s `T` is only in `T::Out`, and that `T::Out` is an `i64` doesn't
// say which `T` it is: both impls give one. Working out a projection only
// goes forward, from the type to what it stands for.
trait Tr {
    type Out;
}

impl Tr for bool {
    type Out = i64;
}

impl Tr for () {
    type Out = i64;
}

fn mk<T: Tr>(x: T::Out) -> T::Out { x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64 = mk(1);
    out.line(&x)?;
    Result::Ok(())
}
// error: cannot infer
