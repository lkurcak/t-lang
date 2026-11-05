// Two closures are of two types, even if neither captures and they look the
// same: a variable holding one can't be given the other. Say it's a
// function, `let mut f: fn(i64) -> i64 = ...`, and it can.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut f = |x: i64| x + 1;
    f = |x: i64| x * 2;
    out.line(&f(1))?;
    Ok(())
}
// error: expected `{closure}`, found `{closure}`; each closure has a type of its own
