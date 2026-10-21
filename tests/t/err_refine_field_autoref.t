// A method taking `&mut self` borrows the field mutably too.
trait Clear {
    fn clear(&mut self);
}

impl Clear for int {
    fn clear(&mut self) { *self = 0; }
}

#[derive(Copy, Discard)]
struct Pos { n: { v: int | v > 0 } }

fn main() {
    let mut p = Pos { n: 1 };
    p.n.clear();
    print(p.n);
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
