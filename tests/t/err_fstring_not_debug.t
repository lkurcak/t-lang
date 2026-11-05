// A hole `{x:?}` writes `x` by its `Debug`, so `x` must have one.
#[derive(Discard)]
struct P { n: i32 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = P { n: 1 };
    out.line(&f"p = {p:?}")?;
    Ok(())
}
// error: `P` doesn't implement `Debug`, which a hole `{x:?}` of a format string needs; add `#[derive(Debug)]` to `P`
