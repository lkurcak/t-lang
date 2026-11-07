// A method taking `&mut self` borrows the field mutably too. This call is
// known to be of `i64`'s impl (step 53), whose `&mut self` is call-scoped,
// so the field can be lent to it, and `Pos`'s refinement is proved again
// after (step 30): which fails, as `clear` makes it 0.
trait Clear {
    fn clear(&mut self);
}

impl Clear for i64 {
    fn clear(&mut self) { *self = 0; }
}

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    p.n.clear();
    out.line(&p.n)?;
    Ok(())
}
// error: can't prove `v > 0` for field `n` of `Pos`
