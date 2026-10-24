// A pattern that binds a refined field by `&mut` borrows it.
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    match &mut p {
        Pos { n } => *n = 0,
    }
    out.line(&p.n)?;
    Result::Ok(())
}
// error: can't bind `n` by `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
