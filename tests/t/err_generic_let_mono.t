// Using a generic function as a value instantiates it once, so `f` has one
// type: there's no let-generalisation.
fn id<T>(x: T) -> T { x }
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = id;
    out.line(&f(1))?;
    out.line(&f(true))?;
    Result::Ok(())
}
// error: expected `{integer}`, found `bool`
