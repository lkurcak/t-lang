// A const has one value on every machine. The greatest `usize` doesn't:
// it's 2^64 - 1 on a 64-bit machine and 2^32 - 1 on a 32-bit one.
const MAX: usize = core::num::greatest::<usize>();
// error: `MAX` is 18446744073709551615 on a 64-bit machine but 4294967295 on a 32-bit one

fn main() {
    let _ = MAX;
}
