// An iterator written through a reference to it is another.
fn main() {
    let a: [u8; 8] = [0; 8];
    let mut it = (0..8).rev();
    let r = &mut it;
    *r = (0..100).rev();
    for x in it {
        let _ = a[x];
    }
}
// error: can't prove
