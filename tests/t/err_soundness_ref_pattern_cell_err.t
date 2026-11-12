// A `&` binding to `Err`'s `Cell`, used where it's read again.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

fn main() {
    let r: Result<u8, Cell<{ d: usize | d < 10 }>> = Err(Cell::new(1));
    match &r {
        Err(c) => {
            c.set(99);
            let _x = read(c);
        }
        Ok(_) => {}
    }
}
// error: can't prove `d < 10` for argument `value` of `set`
