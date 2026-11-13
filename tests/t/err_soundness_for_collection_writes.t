// `for` over a `&mut` to a collection gives a `&mut` to each element,
// whose refinement each write through it must keep, as `iter_mut`'s do.
#![allow(dead_code)]
use alloc::list::List;

fn list(l: &mut List<{ t: usize | t < 5 }>) {
    for x in l {
        *x = 99;
    }
}

fn list_ref(mut l: List<{ t: usize | t < 5 }>) -> List<{ t: usize | t < 5 }> {
    for x in &mut l {
        *x = 99;
    }
    l
}

fn slice(s: &mut [{ t: usize | t < 5 }]) {
    for x in s {
        *x = 99;
    }
}

fn array(a: &mut [{ t: usize | t < 5 }; 3]) {
    for x in a {
        *x = 99;
    }
}

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
