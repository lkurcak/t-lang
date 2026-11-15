// A call through a function value has no type parameters to keep what's
// behind a `&mut` it's given: so it can't be given one to a place whose type
// has refinements of type arguments, which it could write past. (One whose
// value is known is a call of that function: err_soundness_callable_fn_refined.)
use alloc::list::List;

fn push9(l: &mut List<usize>) with alloc { l.push(9) }

fn go(f: fn(&mut List<usize>) with alloc) with alloc {
    let mut q: List<{ t: usize | t < 4 }> = List::new();
    f(&mut q);
}

fn main() with alloc { go(push9) }
// error: can't give `&mut q` to a call through a value
