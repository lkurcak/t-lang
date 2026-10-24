// A `break` leaves the loop with its condition still true.
fn pos(n: { v: i64 | v > 0 }) -> i64 { n }

fn f(stop: bool) -> i64 with panic {
    let mut x = 0;
    while x < 10 {
        if stop {
            break;
        }
        x = x + 1;
    }
    pos(x - 9)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(true))?;
    Result::Ok(())
}
// error: can't prove `v > 0` for argument `n` of `pos`
