// `!0` of a `u8` is 255, not -1. If the checker thought -1, it would take
// this branch never to run, and `x - 1` not to panic.
fn f(x: { v: u8 | v == 0 }) -> u8 with {} {
    if !x == 255 { x - 1 } else { x }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(0))?;
    Result::Ok(())
}
// error: `-` may panic, but `f` doesn't say `with panic`
