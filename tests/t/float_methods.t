// `Float`'s methods, of `f64` and `f32`.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let xs: [f64; 9] = [2.7, -2.7, 2.5, -2.5, 0.5, -0.5, 4503599627370497.0, 1e300, -0.0];
    for x in xs {
        out.line(&f"{x}: {x.floor()} {x.ceil()} {x.trunc()} {x.round()}")?;
    }
    // expect: 2.7: 2.0 3.0 2.0 3.0
    // expect: -2.7: -3.0 -2.0 -2.0 -3.0
    // expect: 2.5: 2.0 3.0 2.0 3.0
    // expect: -2.5: -3.0 -2.0 -2.0 -3.0
    // expect: 0.5: 0.0 1.0 0.0 1.0
    // expect: -0.5: -1.0 -0.0 -0.0 -1.0
    // expect: 4503599627370497.0: 4503599627370497.0 4503599627370497.0 4503599627370497.0 4503599627370497.0
    // expect: 1e300: 1e300 1e300 1e300 1e300
    // expect: -0.0: -0.0 -0.0 -0.0 -0.0
    let nan = f64::nan();
    out.line(&f"{nan.floor()} {f64::infinity().round()}")?; // expect: NaN inf
    out.line(&f"{(0.49999999999999994f64).round()}")?; // expect: 0.0
    out.line(&f"{(-3.5f64).abs()} {(-0.0f64).abs()} {3.0f64.copysign(-1.0)}")?; // expect: 3.5 0.0 -3.0
    out.line(&f"{2.0f64.sqrt()} {2.0f32.sqrt()} {(-1.0f64).sqrt()} {(-0.0f64).sqrt()}")?;
    // expect: 1.4142135623730951 1.4142135 NaN -0.0
    // NaN in, NaN out, of `min` and `max` too; and `-0.0` is less.
    out.line(&f"{1.0f64.min(nan)} {nan.max(1.0)} {0.0f64.min(-0.0)} {(-0.0f64).max(0.0)}")?;
    // expect: NaN NaN -0.0 0.0
    out.line(&f"{1.0f64.min(2.0)} {1.0f32.max(2.0)}")?; // expect: 1.0 2.0
    out.line(&f"{nan.is_nan()} {nan.is_finite()} {f64::infinity().is_infinite()} {1.0f64.is_finite()}")?;
    // expect: true false true true
    out.line(&f"{(-0.0f64).is_sign_negative()} {0.0f64.is_sign_negative()} {(-nan).is_sign_negative()}")?;
    // expect: true false true
    out.line(&f"{f64::greatest()} {f32::greatest()}")?; // expect: 1.7976931348623157e308 3.4028235e38
    out.line(&f"{f64::least_positive()} {f32::least_positive()}")?; // expect: 2.2250738585072014e-308 1.1754944e-38
    out.line(&f"{f64::epsilon()} {f32::epsilon()}")?; // expect: 2.220446049250313e-16 1.1920929e-7
    out.line(&f"{f32::nearest(16777217i32)} {f64::nearest(core::num::greatest::<u64>())}")?;
    // expect: 16777216.0 1.8446744073709552e19
    out.line(&f"{f32::nearest_f64(0.1)} {f32::nearest_f64(1e300)}")?; // expect: 0.1 inf
    Ok(())
}
