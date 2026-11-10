// A write to a field of an element: the element's type, as the list's type
// argument says it, is what the field's new value must have.
#![allow(dead_code)]
use alloc::list::List;

#[derive(Copy, Discard)]
struct W<T> { v: T }

fn tuple(l: &mut List<(u8, { v: usize | v < 5 })>) {
    if l.len() > 0 {
        l[0].1 = 99;
    }
}

fn generic_struct(l: &mut List<W<{ t: usize | t < 5 }>>) {
    if l.len() > 0 {
        l[0].v = 99;
    }
}

fn array(a: &mut [(u8, { v: usize | v < 5 }); 2]) { a[1].1 = 99; }

fn main() {}
// error: can't prove `v < 5`
// error: can't prove `t < 5`
// error: can't prove `v < 5`
