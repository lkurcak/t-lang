// An open end can be narrowed too, keeping the `..=` style.
fn f(n: i64) -> i64 {
    match n {
        0.. => 1,
        ..=10 => 2,
    }
}
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(0))?;
    out.line(&f(-1))?;
    Result::Ok(())
}
// warning: this range can be narrowed to `..=-1`
// expect: 1
// expect: 2
