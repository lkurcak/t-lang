// warning: package `counter` is in `dependencies`, but nothing in this package names it
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&1_i64)?; // expect: 1
    Result::Ok(())
}
