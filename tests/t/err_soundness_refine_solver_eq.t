// `y` is `x + 1`, so `y + x` is `2 * x + 1`, which isn't always -1.
fn minus_one(n: { v: i64 | v == -1 }) -> i64 { n }

fn f(x: i64) -> i64 with panic {
    let y = x + 1;
    minus_one(y + x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&f(1))?;
    Result::Ok(())
}
// error: can't prove `v == -1` for argument `n` of `minus_one`
