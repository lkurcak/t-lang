// A pattern can't bind a field of one by `&mut` either.
fn main() {
    let mut p: { p: (u8, u8) | p.0 < p.1 } = (1, 2);
    let (a, _) = &mut p;
    *a = 9;
}
// error: can't borrow `p` as `&mut`
