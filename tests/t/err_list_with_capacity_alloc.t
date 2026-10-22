// `List::new` allocates nothing, but `with_capacity` does.
use alloc::list::List;

fn empty() -> List<int> { List::new() }

fn two() -> List<int> { List::with_capacity(2) }

fn main() with {alloc, panic} {
    empty().free();
    two().free();
}
// error: `with_capacity` may allocate, but `two` doesn't say `with alloc`
