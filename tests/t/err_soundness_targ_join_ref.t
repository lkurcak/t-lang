// A reference that's one of two, to lists with different refinements: what
// goes through it must have both.
use alloc::list::List;

fn main() with alloc {
    let mut q3: List<{ t: usize | t < 3 }> = List::new();
    let mut q5: List<{ t: usize | t < 5 }> = List::new();
    let c = q5.len() == 0;
    let r = if c { &mut q3 } else { &mut q5 };
    r.push(4);
}
// error: can't prove `t < 3` for argument `v` of `push`
