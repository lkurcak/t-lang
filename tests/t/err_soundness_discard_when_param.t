// A list a function is given may hold anything: it can't drop it.
use alloc::list::List;

fn forget<T>(_l: List<T>) {}

struct Fd { n: i64 }

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    forget(l);
}
// error: `_l` is dropped here, and `List<T>` isn't `Discard`: only `{ l: List<T> | l.len() == 0 }` is, which can't be proved of it
