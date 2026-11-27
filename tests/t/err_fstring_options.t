// Only `:?`, `:x`, `:0Nx`, `:0N` and widths can follow a hole's
// expression, for now: no other radix.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 5u8;
    out.line(&f"x = {x:b}")?;
    Ok(())
}
// error: a hole in a format string can say `:?`, for `Debug`, `:x` or `:08x`, for hexadecimal, at least 8 digits, `:05`, for decimal, at least 5 characters, `:.3`, for a float with 3 digits after its point, and `:<8`, `:^8` or `:>8` for a width
