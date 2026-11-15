// A list that holds a linear value can't be dropped.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
}
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
