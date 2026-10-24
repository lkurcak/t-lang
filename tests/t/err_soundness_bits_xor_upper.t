// `a ^ b` can be more than either: 1 ^ 2 is 3.
fn f(a: { v: u8 | v <= 2 }, b: { v: u8 | v <= 2 }) -> { r: u8 | r <= 2 } { a ^ b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(1, 2))?;
    Result::Ok(())
}
// error: can't prove `r <= 2` for the result of `f`
