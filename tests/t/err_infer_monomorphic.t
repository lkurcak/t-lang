// A variable has one type. `id` is `fn(i64) -> i64` once it's called with an
// i64, so it can't take a bool too. Closures stay monomorphic even with
// generics: only a top-level `fn id<T>(x: T) -> T` can be used at both.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let id = |x| x;
    out.line(&id(1))?;
    out.line(&id(true))?;
    Result::Ok(())
}
// error: expected `{integer}`, found `bool`
