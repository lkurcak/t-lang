// A hole borrows its place shared, so it can't change while the string
// that names it is still used: what's written is what was there.
#[derive(Debug, Discard)]
struct P { x: i32 }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = P { x: 1 };
    let line = f"{p.x}";
    p.x = 2;
    out.line(&line)?;
    Ok(())
}
// error: cannot assign to `p.x` while `p.x` is borrowed
