// Two `&Cell`s of one type parameter: generic code can set one to what
// the other holds.
use core::cell::Cell;

#[derive(Copy, Discard)]
struct P<'a, T> {
    a: &'a T,
    b: &'a T,
}

fn copy_over<'a, U: Copy>(p: P<'a, Cell<U>>) {
    p.a.set(p.b.get());
}

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

fn main() {
    let c1: Cell<{ d: usize | d < 10 }> = Cell::new(1);
    let c2: Cell<usize> = Cell::new(99);
    copy_over(P { a: &c1, b: &c2 });
    let _x = read(&c1);
}
// error: can't prove `d < 10` of what may be written through field `b` of `P`: it's of the same type as field `a` of `P`, which has it
