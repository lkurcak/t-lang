// A result's refinement says what callers may count on, and can name the
// parameters.
fn clamp(x: i64, hi: { h: i64 | h >= 0 }) -> { r: i64 | r <= hi } {
    if x < hi { x } else { hi }
}

fn at_least_one(n: i64) -> { r: i64 | r >= 1 } {
    if n < 1 { 1 } else { n }
}

fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }

fn inc(x: i64) -> { r: i64 | r == x + 1 } with panic { x + 1 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&clamp(12, 10))?; // expect: 10
    out.line(&div(9, at_least_one(-5)))?; // expect: 9
    out.line(&div(9, at_least_one(3)))?; // expect: 3
    let y = inc(4);
    out.line(&div(10, y - 4))?; // expect: 10
    Result::Ok(())
}
