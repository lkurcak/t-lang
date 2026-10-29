// `%` by zero panics, as `/` does.
fn rem(a: i64, b: i64) -> i64 with {} { a % b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&rem(1, 0))?;
    Result::Ok(())
}
// error: `%` may panic, but `rem` doesn't say `with panic`
