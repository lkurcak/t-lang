// In the second arm, `d` is anything but 0, not 0.
fn zero(z: { v: i64 | v == 0 }) -> i64 { z }

fn f(d: i64) -> i64 {
    match d {
        0 => 0,
        _ => zero(d),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(5))?;
    Result::Ok(())
}
// error: can't prove `v == 0` for argument `z` of `zero`
