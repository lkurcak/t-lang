// `Box::new` allocates, and aborts if memory has run out.

use alloc::boxed::Box;

fn boxed(n: i64) -> Box<i64> with panic { Box::new(n) }

fn main() with {alloc, panic} {
    let _v = *boxed(1);
}
// error: `Box::new` may allocate, but `boxed` doesn't say `with alloc`
