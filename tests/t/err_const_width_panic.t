// What fits a 64-bit `usize` but not a 32-bit one panics on the latter.
const N: usize = 65536 * 65536;
// error: evaluating `N` on a 32-bit machine panicked: integer overflow

fn main() {
    let _ = N;
}
