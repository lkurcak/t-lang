fn id<T>(x: T) -> T { x }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = id;
    out.line(&(f::<i64>(1)))?;
    Result::Ok(())
}
// error: `f` is a variable, and only functions, structs and enums take type arguments
