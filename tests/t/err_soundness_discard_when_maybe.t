// Nor one that may: on some paths.
use alloc::list::List;

struct Fd { n: i64 }

fn fill(c: bool) with alloc {
    let mut l: List<Fd> = List::new();
    if c {
        l.push(Fd { n: 1 });
    }
}

fn main() with alloc { fill(true); }
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
