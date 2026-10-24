// A list is `Discard` only if its values are: one of file handles must
// still be used up with `free_with`.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let mut l = List::new();
    l.push(Fd { n: 1 });
}
// error: `l` is never used up, and its type, `List<Fd>`, isn't `Discard`
