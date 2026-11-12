// An array of `&mut`s to two places with different declarations.
use alloc::list::List;
use core::mem::swap;

fn sw<'a, T>(p: [&'a mut T; 2]) {
    let [a, b] = p;
    swap(a, b);
}

fn main() with alloc {
    let mut l: List<{ t: usize | t < 5 }> = List::new();
    l.push(1);
    let mut y: usize = 99;
    if l.len() > 0 {
        sw([&mut l[0], &mut y]);
    }
    if l.len() > 0 {
        let _x: { t: usize | t < 5 } = l[0];
    }
}
// error: can't prove `t < 5` of what may be written through element 1 of the array: it's of the same type as element 0 of the array, which has it
