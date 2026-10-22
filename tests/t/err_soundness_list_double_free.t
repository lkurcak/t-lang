// `free` uses the list up, so it can't be freed twice.
use alloc::list::List;

fn main() {
    let l: List<int> = List::new();
    l.free();
    l.free();
}
// error: use of moved value `l`
