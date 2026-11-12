// A `&` binding into a struct's field's variant.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

#[derive(Discard)]
struct S { o: Option<Cell<{ d: usize | d < 10 }>> }

fn main() {
    let s = S { o: Some(Cell::new(1)) };
    match &s.o {
        Some(c) => c.set(99),
        None => {},
    }
    match s.o.as_ref() {
        Some(c) => {
            let _x = read(c);
        },
        None => {},
    }
}
// error: can't prove `d < 10` for argument `value` of `set`
