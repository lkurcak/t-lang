// A `&mut` to a refined field could be written anything: lent to a call,
// the refinement is proved again when it returns, and `zero` breaks it.
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn zero(r: &mut i64) {
    *r = 0;
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    zero(&mut p.n);
    out.line(&p.n)?;
    Result::Ok(())
}
// error: can't prove `v > 0` for field `n` of `Pos`
