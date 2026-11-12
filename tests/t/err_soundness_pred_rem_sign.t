// A predicate's `%` has the sign of what's divided, as a body's does:
// -7 % 2 is -1. The Euclidean remainder, 1, is what this claims.
fn f(a: { v: i64 | v == -7 }) -> { r: i64 | r % 2 == 1 } { a }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-7))?;
    Ok(())
}
// error: can't prove `r % 2 == 1` for the result of `f`
