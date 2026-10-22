// `push` may grow the buffer, so it allocates, and may panic, out of memory.
use alloc::list::List;

fn add(l: &mut List<i64>) { l.push(1) }

fn main() with {alloc, panic} {
    let mut l: List<i64> = List::new();
    add(&mut l);
    l.free();
}
// error: `push` may allocate, but `add` doesn't say `with alloc`
