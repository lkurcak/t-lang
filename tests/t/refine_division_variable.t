// Dividing by an unknown: `q = a / b` rounds towards zero, so `q * b` is
// within one `b` of `a`, on the side of zero. That proves the usual
// overflow check for a product.

fn times(a: usize, b: { b: usize | b > 0 }) -> Option<usize> with {} {
    if a <= core::num::greatest::<usize>() / b { Some(a * b) } else { None }
}

// Rounded down to a multiple of `b`: no more than `a`, and no more than
// `b - 1` less.
fn floor(a: usize, b: { b: usize | b > 0 }) -> { r: usize | r <= a && a - r < b } with {} {
    a / b * b
}

// Rounded towards zero, a negative number goes up.
fn towards_zero(a: { a: i64 | a < 0 }, b: { b: i64 | b > 0 }) -> { r: i64 | r >= a && r <= 0 }
with {}
{ a / b * b }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&times(6, 7).unwrap_or(0))?;                           // expect: 42
    out.line(&times(core::num::greatest::<usize>(), 2).is_none())?; // expect: true
    out.line(&floor(17, 5))?;                                       // expect: 15
    out.line(&towards_zero(-17, 5))?;                               // expect: -15
    Ok(())
}
