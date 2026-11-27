// The run-time check of `as` to a float: 2^53 + 1 isn't an `f64`, though
// 2^53 and 2^53 + 2 are.
fn f(n: i64) -> f64 with panic { n as f64 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(9007199254740992))?; // expect: 9007199254740992.0
    out.line(&f(9007199254740994))?; // expect: 9007199254740994.0
    out.line(&f(9007199254740993))?;
    Ok(())
}
// error: runtime error: `as` failed: 9007199254740993 isn't an `f64` exactly
