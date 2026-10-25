// `a[0]` is `*a.index(0)`, a place behind a reference: a value that isn't
// `Copy` can't be moved out of it.
use alloc::list::List;

fn main() {
    let a: [List<i64>; 1] = [List::new()];
    let _l = a[0];
}
// error: cannot move out of this value, which is behind a reference: a `List<i64>` isn't `Copy`
