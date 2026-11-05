// Only `:?` can follow a hole's name, for now: no width or radix.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 5u8;
    out.line(&f"x = {x:>4}")?;
    Result::Ok(())
}
// error: a hole in a format string can say `:?`, for `Debug`, and nothing else yet
