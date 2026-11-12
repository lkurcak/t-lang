// `a / -2` is the negation of `a / 2`, though both are of the same
// quotient: 5 / 2 is 2, and 5 / -2 is -2.
fn f(a: { v: i64 | v == 5 }) -> { r: i64 | r == a / -2 } { a / 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    Ok(())
}
// error: can't prove `r == a / -2` for the result of `f`
