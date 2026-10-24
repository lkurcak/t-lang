// `-n` overflows at the smallest `i64`, which has no positive partner.
fn neg(n: i64) -> i64 { -n }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&neg(-9223372036854775807 - 1))?;
    Result::Ok(())
}
// error: `-` may panic, but `neg` doesn't say `with panic`
