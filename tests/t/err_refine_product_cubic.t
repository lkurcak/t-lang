// The logic has products of two unknowns, not three: `x * x * x` fits,
// but the checker can't tell.
fn cube(x: { v: i64 | v >= 0 && v < 10 }) -> i64 with {} { x * x * x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&cube(3))?;
    Ok(())
}
// error: `*` may panic, but `cube` doesn't say `with panic`
