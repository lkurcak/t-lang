// A divisor known to be positive can't be zero, nor -1 under the least
// `i64`, so dividing by it can't panic.
fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&div(10, 3))?; // expect: 3
    let n = 7;
    out.line(&div(-14, n))?; // expect: -2
    let k = div(100, 9);
    if k > 0 {
        out.line(&div(50, k))?; // expect: 4
    }
    Result::Ok(())
}
