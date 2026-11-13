// Only `:?`, `:x` and `:0Nx` can follow a hole's expression, for now: no
// alignment or other radix.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 5u8;
    out.line(&f"x = {x:>4}")?;
    Ok(())
}
// error: a hole in a format string can say `:?`, for `Debug`, or `:x` or `:08x`, for hexadecimal, at least 8 digits, and nothing else yet
