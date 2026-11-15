// A list is `Discard` only if its values are: one of file handles that
// may hold one must still be used up, by `free_with` or a `for`. Only an
// empty one may be dropped (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let mut l = List::new();
    l.push(Fd { n: 1 });
}
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
