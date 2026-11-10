// An element, or a field of one, lent `&mut` to a function that takes a
// `&mut` to a type that says nothing: the function may write anything
// there, so what's declared of the element must be what the parameter's
// type says, as for a lent variable.
#![allow(dead_code)]
use alloc::boxed::Box;
use alloc::list::List;

fn bad(r: &mut usize) { *r = 99 }

#[derive(Discard)]
struct G { l: List<{ t: usize | t < 5 }> }

fn array(a: &mut [{ t: usize | t < 5 }; 2]) { bad(&mut a[0]) }

fn list(l: &mut List<{ t: usize | t < 5 }>) {
    if l.len() > 0 {
        bad(&mut l[0]);
    }
}

fn in_a_field(g: &mut G) {
    if g.l.len() > 0 {
        bad(&mut g.l[0]);
    }
}

fn tuple(t: &mut (u8, { v: usize | v < 5 })) { bad(&mut t.1) }

fn tuple_in_array(a: &mut [(u8, { v: usize | v < 5 }); 1]) { bad(&mut a[0].1) }

fn tuple_in_box(b: &mut Box<(u8, { v: usize | v < 5 })>) { bad(&mut b.1) }

fn tuple_in_option(o: &mut Option<(u8, { v: usize | v < 5 })>) {
    if let Some(p) = o.as_mut() {
        bad(&mut p.1);
    }
}

fn tuple_in_list(l: &mut List<(u8, { v: usize | v < 5 })>) {
    if l.len() > 0 {
        bad(&mut l[0].1);
    }
}

fn main() {}
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `t < 5`
// error: can't prove `v < 5`
// error: can't prove `v < 5`
// error: can't prove `v < 5`
// error: can't prove `v < 5`
// error: can't prove `v < 5`
