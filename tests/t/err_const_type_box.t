// Nor what's on the heap.
use alloc::boxed::Box;

const B: Option<Box<u8>> = Option::None;
// error: a const can't be of type `Option<Box<u8>>`

fn main() {}
