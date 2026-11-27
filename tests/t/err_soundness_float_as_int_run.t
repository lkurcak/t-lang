// The run-time check of `as` from a float: 2.5 isn't an integer, so no
// `i32` is the same number.
fn f(x: f64) -> i32 with panic { x as i32 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(-7.0))?; // expect: -7
    out.line(&f(2.5))?;
    Ok(())
}
// error: runtime error: `as` failed: the `f64` isn't an `i32`
