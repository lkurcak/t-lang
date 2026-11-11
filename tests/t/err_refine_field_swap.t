// `swap` can't break a refined field either: what it may write behind
// each argument is what the other holds, which must have it, and it's
// proved again after.
use core::mem::swap;

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    let mut z = 0;
    swap(&mut p.n, &mut z);
    out.line(&p.n)?;
    Ok(())
}
// error: can't prove `v > 0` of what `swap` may write behind argument `a`
// error: can't prove `v > 0` for field `n` of `Pos`
