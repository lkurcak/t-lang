// That there's a `T` at an address is the caller's to know.
use core::volatile::Volatile;

fn main() {
    let _r: &'static Volatile<u32> = Volatile::at(0x1000);
// error: `at` is unsafe, but it isn't in an `unsafe` block
}
