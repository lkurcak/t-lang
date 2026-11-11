// A pattern that binds a refined field by `&mut` borrows it: the
// reference says the field's refinement, which is about it alone, so what's
// written through it must have it. (Up to 0.80.0 the binding was refused.)
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    match &mut p {
        Pos { n } => *n = 0,
    }
    out.line(&p.n)?;
    Ok(())
}
// error: can't prove `v > 0` for what's written through `*n`
