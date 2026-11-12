// A `&` binding into a variant's field that's a `Cell` says what's declared
// there, which a `set` through it must keep.
use core::cell::Cell;

fn read(o: &Option<Cell<{ d: usize | d < 10 }>>) -> u8 {
    let a = [0u8; 10];
    match o.as_ref() {
        Some(c) => a[c.get()],
        None => 0,
    }
}

fn main() {
    let o: Option<Cell<{ d: usize | d < 10 }>> = Some(Cell::new(1));
    match &o {
        Some(c) => c.set(99),
        None => {}
    }
    let _x = read(&o);
}
// error: can't prove `d < 10` for argument `value` of `set`
