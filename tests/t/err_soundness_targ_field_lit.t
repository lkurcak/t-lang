// A struct's literal must have its fields' refinements, of their type
// arguments too.
use alloc::list::List;

#[derive(Discard)]
struct Bounded { size: usize, items: List<{ t: usize | t < size }> }

fn main() with alloc {
    let mut l: List<usize> = List::new();
    l.push(5);
    let b = Bounded { size: 3, items: l };
    let _ = b;
}
// error: can't prove `t < size` for field `items` of `Bounded`
