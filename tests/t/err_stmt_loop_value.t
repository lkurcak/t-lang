// A `loop` at the start of a statement is the whole statement, as an `if`
// is, so it must be a `()`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    loop { break 5; }
    out.line(&1)?;
    Result::Ok(())
}
// error: expected `()`, found `{integer}`: this `loop` is a statement,
