// The halves of `split_at_mut` borrow the slice while they live.
fn main() {
    let mut a: [u8; 4] = [1, 2, 3, 4];
    let s = a.as_mut_slice();
    if let Some((h, t)) = s.split_at_mut(2) {
        let _n = s.len();
        h[0] = 0;
        t[0] = 0;
    }
}
// error: cannot borrow `*s` while `*s` is mutably borrowed
