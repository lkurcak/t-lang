// The interpreter has no machine, so no addresses: it stops where one is
// named.
use core::volatile::Volatile;

fn main() {
    // SAFETY: a register of a `u32` is at 0x1000, on the board this is for.
    let r: &'static Volatile<u32> = unsafe { Volatile::at(0x1000) };
// error: `Volatile::at` names an address of the machine's, which the interpreter hasn't got
    r.write(7);
}
