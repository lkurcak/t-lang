// A hole holds a place, a name or a field of one, and nothing that would
// run code: a call is worked out in a `let` first.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let s = "abc";
    out.line(&f"{s.len()}")?;
    Result::Ok(())
}
// error: a hole in a format string holds a name, or a field of one, as in `{x}` or `{p.x}`; work out anything else in a `let` first
