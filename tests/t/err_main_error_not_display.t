// The error is written out, so it must say how.
#[derive(Discard)]
struct Oops {}

fn main() -> Result<(), Oops> { Result::Err(Oops {}) }
// error: the error `main` gives back is written out, so it must be `Display`, and a `Oops` isn't
