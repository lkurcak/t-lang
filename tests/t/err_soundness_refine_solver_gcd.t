// `2 * v == 4` has a solution, 2, so it can't prove just anything.
fn negative(n: { v: i64 | v < 0 }) -> i64 { n }

fn f(x: { v: i64 | 2 * v == 4 }) -> i64 { negative(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(2))?;
    Result::Ok(())
}
// error: can't prove `v < 0` for argument `n` of `negative`
