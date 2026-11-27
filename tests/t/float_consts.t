// Consts of floats are worked out while compiling, as IEEE 754 says, the
// same as when the program runs; a NaN keeps its bits.
const TAU: f64 = 6.283185307179586;
const HALF: f32 = 1.0 / 2.0;
const INF: f64 = 1.0 / 0.0;
const SIGNALING: f64 = f64::from_bits(0x7FF0_0000_0000_0001);

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&(TAU / 2.0))?;                           // expect: 3.141592653589793
    out.line(&HALF)?;                                  // expect: 0.5
    out.line(&-INF)?;                                  // expect: -inf
    out.line(&SIGNALING.to_bits())?;                   // expect: 9218868437227405313
    out.line(&(HALF * 2.0))?;                          // expect: 1.0
    out.line(&f64::from_bits(0x3FB9_9999_9999_999A))?; // expect: 0.1
    out.line(&(-1.0f32).to_bits())?;                   // expect: 3212836864
    Ok(())
}
