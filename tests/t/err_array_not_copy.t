// An array is `Copy` only if its values are: this one moves.
use alloc::list::List;

fn main() {
    let a: [List<i64>; 1] = [List::new()];
    let _b = a;
    let _c = a;
}
// error: use of moved value `a`
