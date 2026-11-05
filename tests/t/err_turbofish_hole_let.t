fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x: _ = 1;
    out.line(&x)?;
    Ok(())
}
// error: `_` can only stand for a type in type arguments, like `id::<_>`
