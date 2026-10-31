// A function other than `main` may pass its obligations on to its callers,
// by saying `with unsafe`, which is the fix.
use alloc::raw::RawBuf;

fn first(b: &RawBuf<i64>) -> usize {
    if b.cap() > 0 { b.slice(0, 1).len() } else { 0 }
}

fn main() {}
// error: `slice` is unsafe, but it isn't in an `unsafe` block, and `first` doesn't say `with unsafe`
