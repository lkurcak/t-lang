// A `while` loop can end without its `break`, by its condition, so `x` may
// have no value after it.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: i64;
    let mut i = 0;
    while i < 3 {
        i += 1;
        x = i;
        break;
    }
    out.line(&x)?;
    Result::Ok(())
}
// error: use of `x`, which has no value here
