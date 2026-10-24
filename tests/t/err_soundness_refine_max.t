// Every `i64` is at most the greatest, which isn't enough for `+ 1`.
fn inc(x: { v: i64 | v <= 9223372036854775807 }) -> i64 { x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&inc(9223372036854775807))?;
    Result::Ok(())
}
// error: `+` may panic, but `inc` doesn't say `with panic`
