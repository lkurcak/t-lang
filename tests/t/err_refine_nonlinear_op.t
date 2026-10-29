// True, but the checker only reasons linearly: `x * x` is a product of two
// unknowns, so it can't prove it fits.
fn square(x: { v: i64 | v >= 0 && v < 100 }) -> i64 with {} { x * x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&square(3))?;
    Result::Ok(())
}
// error: `*` may panic, but `square` doesn't say `with panic`
