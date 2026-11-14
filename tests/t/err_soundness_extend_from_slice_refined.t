// Values put in a list of refined values must have its refinement.
use alloc::list::List;

fn main() with alloc {
    let mut digits: List<{ v: u8 | v < 10 }> = List::new();
    let bytes = [200u8];
    digits.extend_from_slice(bytes.as_slice());
}
// error: can't prove `v < 10` for argument `s` of `extend_from_slice`
