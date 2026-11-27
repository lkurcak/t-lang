// Floats as text: `Display` and `Debug` write the fewest digits that read
// back as the same value, as Rust's `{:?}`; `{x:.3}` writes the exact value
// rounded at 3 digits after the point, ties to even.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let xs: [f64; 12] = [0.1, 1.0, 100.0, 1e15, 1e16, 0.0001, 0.00001, 1.5e-7, 123456.789, 5e-324, 1.7976931348623157e308, -2.5];
    for x in xs {
        out.line(&f"{x} {x:?}")?;
    }
    // expect: 0.1 0.1
    // expect: 1.0 1.0
    // expect: 100.0 100.0
    // expect: 1000000000000000.0 1000000000000000.0
    // expect: 1e16 1e16
    // expect: 0.0001 0.0001
    // expect: 1e-5 1e-5
    // expect: 1.5e-7 1.5e-7
    // expect: 123456.789 123456.789
    // expect: 5e-324 5e-324
    // expect: 1.7976931348623157e308 1.7976931348623157e308
    // expect: -2.5 -2.5
    out.line(&f"{1.0f32 / 3.0} {16777216.0f32} {1e-45f32}")?; // expect: 0.33333334 16777216.0 1e-45
    let y = 2.675f64;
    out.line(&f"{y:.2} {0.125f64:.2} {0.375f64:.2} {-0.001f64:.2} {y:.0} {1e21f64:.1}")?;
    // expect: 2.67 0.12 0.38 -0.00 3 1000000000000000000000.0
    out.line(&f"[{y:>8.3}] [{y:<8.1}] [{y:*^9.2}]")?; // expect: [   2.675] [2.7     ] [**2.67***]
    out.line(&f"{f64::nan():.2} {f64::infinity():.3} {1.5f32:.3}")?; // expect: NaN inf 1.500
    Ok(())
}
