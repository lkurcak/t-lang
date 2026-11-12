// `if let` with a `&` binding into a `Cell` in an `Option`.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

fn main() {
    let o: Option<Cell<{ d: usize | d < 10 }>> = Some(Cell::new(1));
    if let Some(c) = &o {
        c.set(99);
    }
    if let Some(c) = o.as_ref() {
        let _x = read(c);
    }
}
// error: can't prove `d < 10` for argument `value` of `set`
