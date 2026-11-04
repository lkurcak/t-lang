// What's there is only read, by a volatile access: never copied out
// whole, which would be a read the program didn't say.
use core::volatile::Volatile;

fn copy(r: &Volatile<u8>) -> Volatile<u8> { *r }
// error: cannot move out of

fn main() {}
