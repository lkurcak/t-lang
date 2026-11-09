// Not in a box, for now: a box of something with no size known when
// compiled would need to be made with that size.
use alloc::boxed::Box;

fn f<unsized D>(x: Box<D>) {}

fn main() {}
// error: `D` is `unsized`, so it can only be behind a reference, as in `&D`
