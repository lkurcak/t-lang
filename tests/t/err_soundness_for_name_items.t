// What a range's `for` gives is below its end only while the iterator is
// changed by `next` alone: given another, it gives what that one does
// (docs/82-item-invariants.md), named or not.
fn main() {
    let a: [u8; 8] = [0; 8];
    for r in it: 0..8 {
        let _ = a[r];
        if r == 3 {
            it = 90..100;
        }
    }
}
// error: can't prove `i < self.len()`
