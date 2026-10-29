// `-x` overflows at the least `i64`, so it needs `x` to be greater.
fn neg(x: { v: i64 | v <= 0 }) -> i64 with {} { -x }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&neg(-9223372036854775807 - 1))?;
    Result::Ok(())
}
// error: `-` may panic, but `neg` doesn't say `with panic`
