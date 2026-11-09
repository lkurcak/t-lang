// Nor can a function give it back as a type that doesn't say them.
use alloc::list::List;

fn leak<'a>(q: &'a mut List<{ t: usize | t < 4 }>) -> &'a mut List<usize> { q }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    leak(&mut q).push(9);
}
// error: can't prove `t < 4` of what may be written through the result of `leak`
