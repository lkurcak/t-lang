// A mask keeps a number only if it's all ones above it: 1 & 6 is 0.
fn f(a: { v: u8 | v <= 6 }) -> { r: u8 | r == a } { a & 6 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(1))?;
    Result::Ok(())
}
// error: can't prove `r == a` for the result of `f`
