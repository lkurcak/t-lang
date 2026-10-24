// `Box::new` may run out of memory, which `panic` doesn't cover.

use alloc::boxed::Box;

fn boxed(n: i64) -> Box<i64> with panic { Box::new(n) }

fn main() with {alloc, panic} {
    print(*boxed(1));
}
// error: `Box::new` may allocate, but `boxed` doesn't say `with alloc`
