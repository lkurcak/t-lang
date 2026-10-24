// Written type arguments are checked like inferred ones.
fn id<T>(x: T) -> T { x }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&(id::<bool>(1)))?;  Result::Ok(()) }
// error: expected `bool`, found `{integer}`
