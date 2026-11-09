// Nor can a field of one be lent `&mut`, which could break it unseen.
fn set(x: &mut u8) { *x = 9; }

fn main() {
    let mut p: { p: (u8, u8) | p.0 < p.1 } = (1, 2);
    set(&mut p.0);
    let _ = p;
}
// error: can't borrow `p.0` as `&mut`
