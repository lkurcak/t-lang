// Wrapping at the machine's width differs too.
const M: usize = core::num::wrapping_sub(0usize, 1);
// error: `M` is 18446744073709551615 on a 64-bit machine but 4294967295 on a 32-bit one

fn main() {
    let _ = M;
}
