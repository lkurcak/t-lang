// `grow` adds slots, and more than a `usize` can count is more memory than
// there is: running out of it, not overflow, so `alloc` and no `panic`.
use alloc::raw::RawBuf;
use core::num::greatest;

fn more(b: &mut RawBuf<u8>) with alloc { b.grow(greatest()) }

fn main() with alloc {
    let mut b: RawBuf<u8> = RawBuf::new(1);
    more(&mut b);
    // SAFETY: nothing was written.
    unsafe { b.free() };
}
// error: runtime error: out of memory for 18446744073709551616 slots
