// `free` uses the list up, so it can't be freed twice.
use alloc::list::List;

fn main() {
    let l: List<i64> = List::new();
    l.discard();
    l.discard();
}
// error: use of moved value `l`
