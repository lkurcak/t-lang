// A method taking `&mut self` borrows the field mutably too.
trait Clear {
    fn clear(&mut self);
}

impl Clear for i64 {
    fn clear(&mut self) {
        *self = 0;
    }
}

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    p.n.clear();
    out.line(&p.n)?;
    Result::Ok(())
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
