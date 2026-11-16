// An iterator assigned another in the loop gives what that one does.
fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = (0..8).rev();
    while let Some(r) = it.next() {
        let _ = a[r];
        if r == 3 {
            it = (90..100).rev();
        }
    }
}
// error: can't prove
