// `y` may be `h`, one row past the last: then `y * w + x` is `w * h` or
// more.
fn cell(
    w: usize,
    h: usize,
    x: { x: usize | x < w },
    y: { y: usize | y <= h },
) -> { i: usize | i < w * h } { y * w + x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&cell(3, 2, 0, 2))?;
    Ok(())
}
// error: can't prove `i < w * h` for the result of `cell`
