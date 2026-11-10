// A closure or a function given to a generic function that calls it with
// a `&mut` to something refined: what it writes there must keep what's
// declared there. A closure written at the call is checked with what the
// callee gives it; one made before, or a function, says what it may write
// in its parameters' types, which must say the same.
#![allow(dead_code)]
use alloc::list::List;

fn apply<T, F: FnMut(&mut T) with alloc + Discard>(r: &mut T, mut f: F) with alloc { f(r) }

fn to_99(acc: u8, x: &mut usize) -> u8 {
    *x = 99;
    acc
}

fn push_99(m: &mut List<usize>) with alloc { m.push(99) }

fn closure_value(l: &mut List<{ t: usize | t < 5 }>) with alloc {
    let f = |m: &mut List<usize>| m.push(99);
    apply(l, f);
}

fn function(l: &mut List<{ t: usize | t < 5 }>) with alloc { apply(l, push_99) }

fn function_pointer(l: &mut List<{ t: usize | t < 5 }>) with alloc {
    let f: fn(&mut List<usize>) with alloc = push_99;
    apply(l, f);
}

fn element(a: &mut [{ t: usize | t < 5 }; 2]) with alloc {
    let f = |x: &mut usize| *x = 99;
    apply(&mut a[0], f);
}

fn folded(l: &mut List<{ t: usize | t < 5 }>) {
    let f = |acc: u8, x: &mut usize| {
        *x = 99;
        acc
    };
    let _ = l.iter_mut().fold(0u8, f);
}

fn folded_function(l: &mut List<{ t: usize | t < 5 }>) { let _ = l.iter_mut().fold(0u8, to_99); }

fn any_of(l: &mut List<{ t: usize | t < 5 }>) {
    let f = |x: &mut usize| {
        *x = 99;
        false
    };
    let _ = l.iter_mut().any(f);
}

// `map` keeps its closure, so it's checked with what it may write, though
// it can't count on reading it.
fn mapped(l: &mut List<{ t: usize | t < 5 }>) {
    for _ in l.iter_mut().map(|x| {
        *x = 99;
        0u8
    }) {}
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
