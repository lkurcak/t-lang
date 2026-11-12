// An or-pattern's `&` binding is to one of two places: what's set through
// it must have what's declared at each.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

#[derive(Discard)]
enum E {
    A(Cell<{ d: usize | d < 10 }>),
    B(Cell<usize>),
}

fn r2(e: &E) -> u8 {
    match e {
        E::A(c) => read(c),
        E::B(_) => 0,
    }
}

fn main() {
    let e = E::A(Cell::new(1));
    match &e {
        E::A(c) | E::B(c) => c.set(99),
    }
    let _x = r2(&e);
}
// error: can't prove `d < 10` for argument `value` of `set`
