// What `replace` gives back through a `&mut` is the list as it was: it
// may hold values.
use alloc::list::List;

struct Fd { n: i64 }

fn reset(l: &mut List<Fd>) {
    let old = core::mem::replace(l, List::new());
}

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    reset(&mut l);
    l.free_with(|fd| { let Fd { n: _ } = fd; });
}
// error: `old` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
