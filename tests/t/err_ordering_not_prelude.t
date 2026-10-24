// `Ordering` isn't in the prelude.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let o = 1i64.cmp(&2);
    match o {
        Ordering::Less => out.line(&1)?,
        _ => out.line(&2)?,
    }
    Result::Ok(())
}
// error: unknown type `Ordering`
