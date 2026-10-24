// `+=` overflows as `+` does, so it may panic too. (Step 7 missed it.)
fn bump(x: i64) -> i64 {
    let mut y = x;
    y += 1;
    y
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&bump(9223372036854775807))?;
    Result::Ok(())
}
// error: `+=` may panic, but `bump` doesn't say `with panic`
