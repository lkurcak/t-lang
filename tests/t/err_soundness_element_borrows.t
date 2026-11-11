// A reference to an element of a list or an array whose elements are
// refined says what's declared of the element: what's written through it
// must have it, however the reference is kept or passed on (before step 63
// `&mut l[0]` was a reference nothing was declared behind, and each of
// these wrote 99 among numbers below 5).
#![allow(dead_code)]
use alloc::list::List;
use core::cell::Cell;

fn list_reborrow(l: &mut List<{ t: usize | t < 5 }>) {
    if l.len() > 0 {
        let r = &mut l[0];
        *r = 99;
    }
}

fn array_reborrow(a: &mut [{ t: usize | t < 5 }; 2]) {
    let r = &mut a[0];
    *r = 99;
}

fn annotated(a: &mut [{ t: usize | t < 5 }; 2]) {
    let r: &mut usize = &mut a[1];
    *r = 99;
}

fn passed_on(a: &mut [{ t: usize | t < 5 }; 2]) {
    let r = &mut a[0];
    let q: &mut usize = r;
    *q = 99;
}

fn through_a_reference(a: &mut [{ t: usize | t < 5 }; 2]) {
    let r = &mut *a;
    let q = &mut r[0];
    *q = 99;
}

fn of_a_slice(l: &mut List<{ t: usize | t < 5 }>) {
    let s = l.as_mut_slice();
    if s.len() > 0 {
        let q = &mut s[0];
        *q = 99;
    }
}

fn returned(a: &mut [{ t: usize | t < 5 }; 2]) -> &mut usize {
    &mut a[0]
}

fn a_cell(cs: &[Cell<{ t: usize | t < 5 }>; 1]) {
    let c: &Cell<usize> = &cs[0];
    c.set(99);
}

// A call through a closure value can't be given one at all (docs/60).
fn to_a_closure(a: &mut [{ t: usize | t < 5 }; 2]) {
    let f = |r: &mut usize| *r = 99;
    f(&mut a[0]);
}

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't give `&mut a[0]` to a call through a value
