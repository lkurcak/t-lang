// Putting a value in an arena allocates.
use alloc::arena::Arena;

fn put(a: &Arena<i64>) with {} {
    let _ = a.alloc(1);
}

fn main() {}
// error: `alloc` may allocate, but `put` doesn't say `with alloc`
