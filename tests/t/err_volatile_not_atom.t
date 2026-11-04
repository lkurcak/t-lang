// Only what the machine reads and writes in one access can be volatile.
use core::volatile::Volatile;

fn f(r: &Volatile<Option<u8>>) {}
// error: Atom

fn main() {}
