// Unlike C, there's no "truthiness": the condition must be a bool.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    if 1 { out.line(&1)? }
    Ok(())
}
// error: expected `bool`, found `{integer}`
