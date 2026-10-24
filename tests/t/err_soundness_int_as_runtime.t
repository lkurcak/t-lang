// The run-time check of `as`: -1 isn't a `u64`.
fn f(n: i64) -> u64 with panic { n as u64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(-1))?;
    Result::Ok(())
}
// error: runtime error: `as` overflowed: -1 isn't a `u64`
