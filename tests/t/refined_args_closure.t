// A closure given to a generic function, whose parameter's type is a type
// parameter of it that no closure gives back, can count on what the
// function's other arguments' refinements say of it: the function can only
// pass it values it was given (docs/60-refined-type-arguments.md).
use alloc::list::List;
use core::num::wrapping_add;

fn each<T: Copy, F: FnMut(T) + Discard>(l: &List<T>, mut f: F) {
    for x in l.iter() {
        f(*x);
    }
}

fn sum(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    let mut s: u64 = 0;
    each(l, |i| s = wrapping_add(s, a[i]));
    s
}

// What `fold`'s closure is given is what the iterator gives,
// `<Iter<'_, T> as Iterator>::Item`, a `&T`: the impl `select` finds is
// generic in `T`, so it can only give those. (`map`'s and `filter`'s
// closures can't count on it: those give the closure back, inside the
// adapter, and whoever has it could call it with anything.)
fn folded(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    l.iter().fold(0, |s, i| wrapping_add(s, a[*i]))
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let mut l: List<{ t: usize | t < 4 }> = List::new();
    l.push(3);
    l.push(0);
    out.line(&sum(a, &l))?; // expect: 1001
    out.line(&folded(a, &l))?; // expect: 1001
    Ok(())
}
