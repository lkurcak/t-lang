// `swap` can't get at a refined field either.
use core::mem::swap;

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    let mut z = 0;
    swap(&mut p.n, &mut z);
    out.line(&p.n)?;
    Result::Ok(())
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
