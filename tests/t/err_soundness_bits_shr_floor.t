// `>>` rounds down, not towards zero: -1 >> 1 is -1.
fn f(a: { v: i64 | v == -1 }) -> { r: i64 | r == 0 } { a >> 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(-1))?;
    Result::Ok(())
}
// error: can't prove `r == 0` for the result of `f`
