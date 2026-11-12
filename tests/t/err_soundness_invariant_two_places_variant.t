// A variant's two `&mut` fields of one type parameter.
use alloc::list::List;
use core::mem::swap;

#[derive(Discard)]
enum Two<'a, T> {
    Both(&'a mut T, &'a mut T),
    Neither,
}

fn sw<'a, T>(t: Two<'a, T>) {
    match t {
        Two::Both(a, b) => swap(a, b),
        Two::Neither => {}
    }
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let mut y: usize = 99;
    if l.len() > 0 {
        sw(Two::Both(&mut l[0], &mut y));
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through field 1 of `Two`: it's of the same type as field 0 of `Two`, which has it
