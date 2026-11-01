// Running out of memory in a `#[blame_caller]` function is its caller's,
// as a panic is: here, alloc's `List::with_capacity` blames `make`'s
// line, and `make` blames `main`'s.
use alloc::list::List;
use core::num::greatest;

#[blame_caller]
fn make() -> List<i64> with alloc { List::with_capacity(greatest()) }

fn main() with alloc {
    let _ = make();
}
// error: blame_caller_oom.t:11:13
