// `a[0]` is `*a.index(0)`, a place behind a reference: a value that isn't
// `Copy` can't be moved out of it.
use alloc::list::List;

fn main() with alloc {
    let a: [List<i64>; 1] = [List::new()];
    let _l = a[0];
}
// error: cannot move out of `*a.index(0)`, which is behind a reference
