// What's copied must be as long as what it's copied into: nothing says
// these two are.
fn copy(to: &mut [u8], from: &[u8]) {
    to.copy_from_slice(from);
}

fn main() {
    let mut a = [0u8; 2];
    copy(a.as_mut_slice(), [1u8, 2, 3].as_slice());
}
// error: can't prove `s.len() == self.len()` for argument `src` of `copy_from_slice`
