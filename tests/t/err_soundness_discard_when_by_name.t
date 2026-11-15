// Nor can its `discard` be called by name.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let l: List<Fd> = List::new();
    l.discard();
}
// error: `List<Fd>` doesn't implement `Discard`
