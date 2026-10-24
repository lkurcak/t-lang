// Asking for more memory than there is runs out of it: an abort, which
// `alloc` says may happen, with no `panic`.
use alloc::raw::RawBuf;
use core::num::greatest;

fn huge() -> RawBuf<i64> with alloc { RawBuf::new(greatest()) }

fn main() with alloc {
    let b = huge();
    // SAFETY: nothing was written.
    unsafe { b.free() };
}
// error: runtime error: out of memory for 18446744073709551615 slots
