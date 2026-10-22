// A `&mut` to a refined field could be written anything.
#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn zero(r: &mut i64) { *r = 0; }

fn main() {
    let mut p = Pos { n: 1 };
    zero(&mut p.n);
    print(p.n);
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
