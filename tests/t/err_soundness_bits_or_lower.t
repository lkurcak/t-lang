// `a | b >= b` only when both are non-negative: -1 | 5 is -1.
fn f(a: { v: i8 | v == -1 }, b: { v: i8 | v == 5 }) -> { r: i8 | r >= 5 } { a | b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, 5))?;
    Result::Ok(())
}
// error: can't prove `r >= 5` for the result of `f`
