// A const's value is worked out while compiling, where there's no heap:
// its initializer can't allocate, not even what it frees again.
use alloc::boxed::Box;

fn size() -> usize with alloc {
    let b = Box::new(3usize);
    *b
}

const N: usize = size();
// error: a const's initializer can't have the effect `alloc`

fn main() {}
