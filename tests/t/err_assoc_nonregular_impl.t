// `S<T>` holds a box of a `T::Out`, and `<W<U> as Tr>::Out` is an
// `S<W<W<U>>>`: so `S<W<i64>>` holds an `S<W<W<i64>>>`, which holds an
// `S<W<W<W<i64>>>>`, and so on. Each is finite, through a box, but there's
// no end of them to monomorphise.

use alloc::boxed::Box;

#[derive(Discard)]
struct W<T> { t: T }

trait Tr {
    type Out;
}

struct S<T: Tr> { b: Box<T::Out> }

impl<U> Tr for W<U> {
    type Out = S<W<W<U>>>;
}

fn size(_s: &S<W<i64>>) -> i64 { 0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1)?;
    Result::Ok(())
}
// error: polymorphic recursion
