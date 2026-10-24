// `d` is 1 before the loop, but not each time round it: the body changes it.
fn pos(d: { v: i64 | v > 0 }) -> i64 { d }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut d = 1;
    let mut n: i64 = 3;
    while n > 0 {
        out.line(&pos(d))?;
        d = d - 1;
        n = n - 1;
    }
    Result::Ok(())
}
// error: can't prove `v > 0` for argument `d` of `pos`
