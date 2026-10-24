fn id<T>(x: T) -> T { x }
fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&(id::<i64, bool>(1)))?;  Result::Ok(()) }
// error: `id` takes 1 type argument but 2 were supplied
