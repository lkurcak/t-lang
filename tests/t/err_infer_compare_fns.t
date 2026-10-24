// `==` on a type not known yet is checked once it is.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let eq = |a, b| a == b;
    out.line(&eq(main, main))?;
    Result::Ok(())
}
// error: cannot compare functions with `==`
