fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x = 1u8;
    out.line(&f"a {x")?;
    Result::Ok(())
}
// error: this hole is never closed; write `{{` for the character
