// Given its only value by assignment, a variable needn't be `mut`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut r: i64;
    r = 1;
    out.line(&r)?;       // expect: 1
    Result::Ok(())
}
// warning: variable `r` doesn't need to be mutable
