// 0xFF keeps only numbers up to 0xFF: 300 & 255 is 44.
fn f(a: { v: u16 | v <= 300 }) -> { r: u16 | r == a } { a & 0xFF }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(300))?;
    Result::Ok(())
}
// error: can't prove `r == a` for the result of `f`
