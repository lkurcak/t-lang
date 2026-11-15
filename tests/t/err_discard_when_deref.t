// Writing through a reference drops what was there: a list of linear
// values, which may hold some.
use alloc::list::List;

struct Fd { n: i64 }

fn reset(r: &mut List<Fd>) with alloc {
    *r = List::new();
}

fn main() {}
// error: `*r` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
