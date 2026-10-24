// Nothing says `k` is positive, so the call can't be checked.
fn div(a: i64, d: { d: i64 | d > 0 }) -> i64 { a / d }

fn pick() -> i64 { 3 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let k = pick();
    out.line(&div(1, k))?;
    Result::Ok(())
}
// error: can't prove `d > 0` for argument `d` of `div`
