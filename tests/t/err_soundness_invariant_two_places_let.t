// The same, through a `let` of the struct first.
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
        let w = W2 { a: &mut y, b: &mut l[0] };
        sw(w);
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through field `a` of `W2`: it's of the same type as field `b` of `W2`, which has it
