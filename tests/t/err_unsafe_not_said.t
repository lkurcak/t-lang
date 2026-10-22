// A function other than `main` may pass its obligations on to its callers,
// by saying `with unsafe`, which is the fix.
use alloc::raw::RawBuf;

fn first(b: &RawBuf<int>) -> int { *b.get(0) }

fn main() {}
// error: `get` is unsafe, but it isn't in an `unsafe` block, and `first` doesn't say `with unsafe`
