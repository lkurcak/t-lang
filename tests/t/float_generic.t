// Code over `F: Float` is written once, for both types: its operators,
// literals, comparisons and methods are `F`'s.
fn dot<F: Float>(a: &[F], b: &[F]) -> F {
    let mut sum = 0.0;
    let n = if a.len() < b.len() { a.len() } else { b.len() };
    for i in 0..n {
        sum = sum + a[i] * b[i];
    }
    sum
}

/// The greatest of them, or minus infinity of none: a NaN wins.
fn most<F: Float>(xs: &[F]) -> F {
    let mut m = -F::infinity();
    for x in xs {
        m = m.max(*x);
    }
    m
}

/// Into an `f64`, which every `F` is exactly: so it can't panic.
fn widen<F: Float>(x: F) -> f64 with {} { x as f64 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let a: [f64; 3] = [1.0, 2.0, 3.0];
    let b: [f32; 3] = [0.5, 0.25, 0.125];
    out.line(&dot(&a, &a))?;      // expect: 14.0
    out.line(&dot(&b, &b))?;      // expect: 0.328125
    out.line(&most(&a))?;         // expect: 3.0
    out.line(&most::<f32>(&[]))?; // expect: -inf
    out.line(&widen(0.1f32))?;    // expect: 0.10000000149011612
    Ok(())
}
