// A method taking `&mut self` whose borrow outlives the call can't be
// lent a field a refinement is about: a write through what it gives
// would go unchecked.
trait Hold {
    fn hold(&mut self) -> &mut i64;
}

impl Hold for i64 {
    fn hold(&mut self) -> &mut i64 { self }
}

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut p = Pos { n: 1 };
    let r = p.n.hold();
    *r = 0;
    out.line(&p.n)?;
    Ok(())
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
