fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&f"a } b")?;
    Ok(())
}
// error: a `}` in a format string closes a hole; write `}}` for the character
