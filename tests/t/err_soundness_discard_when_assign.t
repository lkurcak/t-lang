// Overwriting a list throws away what it held.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l = List::new();
    l.free_with(|fd| { let Fd { n: _ } = fd; });
}
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
