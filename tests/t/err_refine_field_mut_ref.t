// A `&mut` to a refined field could be written anything.
#[derive(Copy, Discard)]
struct Pos { n: { v: int | v > 0 } }

fn zero(r: &mut int) { *r = 0; }

fn main() {
    let mut p = Pos { n: 1 };
    zero(&mut p.n);
    print(p.n);
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
