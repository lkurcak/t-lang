// Two type parameters are two different types, even if a caller picks the
// same type for both.
fn first<A, B>(a: A, b: B) -> A { b }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&first(1, 2))?;  Result::Ok(()) }
// error: expected `A`, found `B`
