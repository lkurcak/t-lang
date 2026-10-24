// `a ^ b` is negative when their signs differ: 1 ^ -1 is -2.
fn f(a: { v: i8 | v >= 0 }, b: i8) -> { r: i8 | r >= 0 } { a ^ b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(1, -1))?;
    Result::Ok(())
}
// error: can't prove `r >= 0` for the result of `f`
