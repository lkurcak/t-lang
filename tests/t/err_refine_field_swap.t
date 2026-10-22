// `swap` can't get at a refined field either.
use core::mem::swap;

#[derive(Copy, Discard)]
struct Pos { n: { v: i64 | v > 0 } }

fn main() {
    let mut p = Pos { n: 1 };
    let mut z = 0;
    swap(&mut p.n, &mut z);
    print(p.n);
}
// error: can't borrow `p.n` as `&mut`: field `n` of `Pos` has a refinement, which a write through the reference could break
