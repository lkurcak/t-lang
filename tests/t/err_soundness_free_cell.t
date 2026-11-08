// A `Cell` can give back a value put in it before the call.
use core::cell::Cell;

fn swap_in<T: Copy>(c: &Cell<T>, v: T) -> T { c.replace(v) }

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 {
    let c = Cell::new(9usize);
    a[swap_in(&c, i)]
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
