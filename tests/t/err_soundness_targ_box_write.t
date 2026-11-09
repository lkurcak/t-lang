// What's written into a refined box must have its refinement.
use alloc::boxed::Box;

fn main() with alloc {
    let mut b: Box<{ v: usize | v < 4 }> = Box::new(1);
    *b = 9;
    let _ = b;
}
// error: can't prove `v < 4` for
