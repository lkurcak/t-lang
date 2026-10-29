// `List::new` allocates nothing, but `with_capacity` does.
use alloc::list::List;

fn empty() -> List<i64> { List::new() }

fn two() -> List<i64> with {} { List::with_capacity(2) }

fn main() with {alloc, panic} {
    empty().discard();
    two().discard();
}
// error: `with_capacity` may allocate, but `two` doesn't say `with alloc`
