// A hole `{x}` writes `x` by its `Display`, so `x` must have one; the
// error is at the hole.
#[derive(Debug, Discard)]
struct P { n: i32 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 1 };
    out.line(&f"p = {p}")?;
    Ok(())
}
// error: `P` doesn't implement `Display`, which a hole `{x}` of a format string needs
