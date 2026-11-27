// The run-time check of `as` from `f64` to `f32`: 0.5 and a NaN are
// `f32`s, 0.1 isn't.
fn f(x: f64) -> f32 with panic { x as f32 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(0.5))?;              // expect: 0.5
    out.line(&f(f64::nan()))?;       // expect: NaN
    out.line(&f(-f64::infinity()))?; // expect: -inf
    out.line(&f(0.1))?;
    Ok(())
}
// error: runtime error: `as` failed: the `f64` isn't an `f32`
