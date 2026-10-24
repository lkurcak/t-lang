// Running out of memory is an abort that only `alloc` allows: a function
// saying just `with panic` can't, as it'd abort in a caller that counts on
// it allocating nothing.
use alloc::raw::out_of_memory;

fn spin() -> ! with panic { out_of_memory() }

fn main() with panic {
    spin();
}
// error: `out_of_memory` may allocate, but `spin` doesn't say `with alloc`
