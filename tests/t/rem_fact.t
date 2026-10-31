// The checker knows what any `%` gives, by any divisor: it's less than the
// divisor from 0, no further from 0 than what's divided, and has its sign.
fn slot(h: usize, n: { n: usize | n > 0 }) -> { i: usize | i < n } {
    h % n
}

fn rem(a: i64, b: { b: i64 | b > 0 }) -> { r: i64 | -b < r && r < b } {
    a % b
}

fn rem_natural(a: { a: i64 | a >= 0 }, b: { b: i64 | b > 0 }) -> { r: i64 | 0 <= r && r <= a } {
    a % b
}

fn rem_negative(a: { a: i64 | a <= 0 }, b: { b: i64 | b < -1 }) -> { r: i64 | b < r && r <= 0 } {
    a % b
}

fn index(xs: &[u8; 4], k: u32) -> u8 {
    let n: u32 = 4;
    xs[(k % n) as usize]
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&slot(17, 5))?; // expect: 2
    out.line(&rem(-7, 3))?; // expect: -1
    out.line(&rem_natural(7, 3))?; // expect: 1
    out.line(&rem_negative(-7, -3))?; // expect: -1
    out.line(&index(&[1, 2, 3, 4], 6))?; // expect: 3
    Result::Ok(())
}
