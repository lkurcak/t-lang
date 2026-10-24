// `a & b <= a` only when `a >= 0`: -1 & 5 is 5.
fn f(a: { v: i8 | v == -1 }, b: { v: i8 | v == 5 }) -> { r: i8 | r <= -1 } { a & b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1, 5))?;
    Result::Ok(())
}
// error: can't prove `r <= -1` for the result of `f`
