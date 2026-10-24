// Dividing by a positive constant is linear: the checker knows the
// quotient is within one divisor of what's divided, rounding towards
// zero, and a remainder is less than the divisor from zero.
fn mean(a: u8, b: u8) -> u8 {
    let sum = a as u16 + b as u16;
    (sum / 2) as u8
}

fn half_neg(a: { v: i64 | v == -1 }) -> { r: i64 | r == 0 } { a / 2 }

fn digit(n: u64) -> u8 { (n % 10) as u8 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&mean(200, 255))?;  // expect: 227
    out.line(&half_neg(-1))?;    // expect: 0
    out.line(&digit(1234))?;     // expect: 4
    Result::Ok(())
}
