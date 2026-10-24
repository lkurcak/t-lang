// ...and it can't need more of `T` than the trait says, as callers go by
// the trait's signature.
struct Fd { n: i64 }

trait Sink {
    fn put<T>(&self, x: T) -> i64;
}

#[derive(Discard)]
struct S {}

impl Sink for S {
    fn put<T: Discard>(&self, _x: T) -> i64 { 0 }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(S {}.put(Fd { n: 1 })))?;
    Result::Ok(())
}
// error: method `put` doesn't match its declaration in trait `Sink`: its bounds should be none
