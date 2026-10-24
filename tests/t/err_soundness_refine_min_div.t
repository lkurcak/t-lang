// `d != 0` isn't enough: the least `i64` divided by -1 overflows.
fn div(a: i64, d: { d: i64 | d != 0 }) -> i64 { a / d }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&div(-9223372036854775807 - 1, -1))?;
    Result::Ok(())
}
// error: `/` may panic, but `div` doesn't say `with panic`
