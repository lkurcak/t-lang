// A call through a function value has no type parameters to keep what's
// behind a `&mut` it's given: so it can't be given one to a place whose type
// has refinements of type arguments, which it could write past.
use alloc::list::List;

fn push9(l: &mut List<usize>) with alloc { l.push(9) }

fn main() with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    let f: fn(&mut List<usize>) with alloc = push9;
    f(&mut q);
}
// error: can't give `&mut q` to a call through a value
