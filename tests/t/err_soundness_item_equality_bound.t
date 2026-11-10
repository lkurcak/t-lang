// A generic function whose iterator's items are pinned by an equality
// bound to `&mut usize` may write any `usize` through them: given an
// iterator over a refined list, the call must be refused.
#![allow(dead_code)]
use alloc::list::List;

fn all_to<'a, I: Iterator<Item = &'a mut usize> + Discard>(it: I) {
    for r in it {
        *r = 99;
    }
}

fn first_to<'a, I: Iterator<Item = &'a mut usize> + Discard>(mut it: I) {
    if let Some(r) = it.next() {
        *r = 99;
    }
}

fn list(l: &mut List<{ t: usize | t < 5 }>) { all_to(l.iter_mut()) }

fn array(a: &mut [{ t: usize | t < 5 }; 2]) { first_to(a.iter_mut()) }

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
