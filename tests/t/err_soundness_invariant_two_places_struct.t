// A struct literal holding `&mut`s to two places of one type parameter:
// what's written through either must have what's declared at both.
use alloc::list::List;
use core::mem::swap;

#[derive(Discard)]
struct W2<'a, T> {
    a: &'a mut T,
    b: &'a mut T,
}

fn sw<'a, T>(w: W2<'a, T>) {
    swap(w.a, w.b);
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let mut y: usize = 99;
    if l.len() > 0 {
        sw(W2 { a: &mut l[0], b: &mut y });
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through field `b` of `W2`: it's of the same type as field `a` of `W2`, which has it
