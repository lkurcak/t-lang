// Values copied into a slice of refined values must have its refinement:
// any `u8` isn't a digit.
fn copy(to: &mut [{ v: u8 | v < 10 }], from: { s: &[u8] | s.len() == to.len() }) {
    to.copy_from_slice(from);
}

fn main() {
    let mut digits: [{ v: u8 | v < 10 }; 1] = [0];
    copy(digits.as_mut_slice(), [200u8].as_slice());
    let _ = digits[0] + 250;
}
// error: can't prove `v < 10` for argument `src` of `copy_from_slice`
