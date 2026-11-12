// A `&` binding to an array's element that's a `Cell`.
use core::cell::Cell;

fn read(c: &Cell<{ d: usize | d < 10 }>) -> u8 {
    let a = [0u8; 10];
    a[c.get()]
}

fn main() {
    let arr: [Cell<{ d: usize | d < 10 }>; 2] = [Cell::new(1), Cell::new(2)];
    let [c, _] = &arr;
    c.set(99);
    let _x = read(&arr[0]);
}
// error: can't prove `d < 10` for argument `value` of `set`
