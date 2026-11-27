// A NaN is no number, and an infinity none of an integer type's.
fn f(x: f64) -> u8 with panic { x as u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(255.0))?; // expect: 255
    out.line(&f(-0.0))?; // expect: 0
    out.line(&f(0.0 / 0.0))?;
    Ok(())
}
// error: runtime error: `as` failed: the `f64` isn't a `u8`
