// A quotient in a predicate is an integer, with no type's range: 255 *
// 10^19 / 2 is more than any integer type has room for. Were it one's,
// that would contradict what's known, which proves anything.
fn f(x: { v: u8 | v == 255 && (v * 10000000000000000000) / 2 == 255 * 5000000000000000000 }) -> {
    r: u8 | r == 0
} {
    x
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f(255))?;
    Ok(())
}
// error: can't prove `r == 0` for the result of `f`
