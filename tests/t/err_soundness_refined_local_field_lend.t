// Nor can a field of one be lent `&mut`: a write through it would go
// unseen.
#[derive(Copy, Discard)]
struct P { a: usize, b: usize }

fn set(x: &mut usize) { *x = 9; }

fn main() {
    let mut p: { p: P | p.a < 3 } = P { a: 1, b: 2 };
    set(&mut p.a);
    let _ = p;
}
// error: can't borrow `p.a` as `&mut`: it's in `p`, whose type has a refinement
