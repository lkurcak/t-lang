// IEEE 754's comparisons: a NaN is equal to nothing, itself included, and
// neither less nor greater than anything; `0.0 == -0.0`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let nan = f64::nan();
    out.line(&(nan == nan))?; // expect: false
    out.line(&(nan != nan))?; // expect: true
    out.line(&(nan < 1.0))?; // expect: false
    out.line(&(nan >= 1.0))?; // expect: false
    out.line(&(0.0f64 == -0.0))?; // expect: true
    out.line(&(-0.0f64 < 0.0))?; // expect: false
    out.line(&(f64::infinity() > f64::greatest()))?; // expect: true
    out.line(&(1.5f32 <= 1.5))?; // expect: true
    // Of a type not known when it's compared, a float's comparison is
    // still IEEE 754's, needing neither `Eq` nor `Ord`.
    let lt = |a, b| a < b;
    let eq = |a, b| a == b;
    out.line(&lt(1.0f64, 2.0))?; // expect: true
    out.line(&eq(nan, nan))?; // expect: false
    // `total_cmp` orders every value, NaNs too.
    out.line(&f"{(-0.0f64).total_cmp(&0.0):?}")?; // expect: Less
    out.line(&f"{nan.total_cmp(&f64::infinity()):?}")?; // expect: Greater
    out.line(&f"{(-nan).total_cmp(&-f64::infinity()):?}")?; // expect: Less
    out.line(&f"{2.5f32.total_cmp(&2.5):?}")?; // expect: Equal
    Ok(())
}
