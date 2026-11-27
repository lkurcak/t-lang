// IEEE 754's arithmetic: each operation rounds its exact result once, to
// the nearest value, ties to even; too big is an infinity, no answer a
// NaN, and none of it panics, so a function of it has no effect.
fn mean(a: f64, b: f64) -> f64 with {} { (a + b) / 2.0 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(0.1 + 0.2f64))?;                // expect: 0.30000000000000004
    out.line(&(0.1 + 0.2f32))?;                // expect: 0.3
    out.line(&mean(1.0, 2.0))?;                // expect: 1.5
    out.line(&(1.0f64 / 0.0))?;                // expect: inf
    out.line(&(-1.0f64 / 0.0))?;               // expect: -inf
    out.line(&(1.0f64 / -0.0))?;               // expect: -inf
    out.line(&(0.0f64 / 0.0))?;                // expect: NaN
    out.line(&(f64::greatest() * 2.0))?;       // expect: inf
    out.line(&(f64::least_positive() / 4.0))?; // expect: 5.562684646268003e-309
    out.line(&(5e-324f64 / 2.0))?;             // expect: 0.0
    out.line(&-0.0f64)?;                       // expect: -0.0
    out.line(&(0.0f64 - 0.0))?;                // expect: 0.0
    out.line(&(-0.0f64 + 0.0))?;               // expect: 0.0
    out.line(&(-0.0f64 - 0.0))?;               // expect: -0.0
    let mut x = 1.0f64;
    x += 0.5;
    x *= 4.0;
    x -= 1.0;
    x /= 2.0;
    out.line(&x)?; // expect: 2.5
    // An `f32` literal is the `f32` nearest it, not an `f64` rounded again.
    out.line(&(0.1f32 as f64))?; // expect: 0.10000000149011612
    out.line(&16777217.0f32)?;   // expect: 16777216.0
    Ok(())
}
