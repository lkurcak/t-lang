// A device changes what's at its address on its own: nothing is known of a
// read, even right after a write.
use core::volatile::Volatile;

fn one(_x: { x: u8 | x == 1 }) {}

fn f(r: &Volatile<u8>) {
    r.write(1);
    one(r.read());
// error: can't prove `x == 1` for argument `_x` of `one`
}

fn main() {}
