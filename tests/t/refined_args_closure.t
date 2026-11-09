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

// Iterator adapters too: what `map`'s and `filter`'s closures are given is
// what the iterator gives, `<Iter<'_, T> as Iterator>::Item`, a `&T`: the
// impl `select` finds is generic in `T`, so it can only give those.
fn mapped(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> u64 {
    l.iter().map(|i| a[*i]).fold(0, |x, y| wrapping_add(x, y))
}

fn big(a: [u64; 4], l: &List<{ t: usize | t < 4 }>) -> usize with panic {
    l.iter().filter(|i| a[**i] > 10).count()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let mut l: List<{ t: usize | t < 4 }> = List::new();
    l.push(3);
    l.push(0);
    out.line(&sum(a, &l))?; // expect: 1001
    out.line(&mapped(a, &l))?; // expect: 1001
    out.line(&big(a, &l))?; // expect: 1
    Ok(())
}
