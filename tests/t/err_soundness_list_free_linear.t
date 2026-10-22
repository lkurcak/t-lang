// `free` drops what's in the list, so its values must be `Discard`; a list
// of linear ones is used up by `free_with`.
use alloc::list::List;

struct Fd { n: int }

fn main() with {alloc, panic} {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.free();
}
// error: `List<T>::free` needs its `T` to implement `Discard`, and `Fd` doesn't; add `#[derive(Discard)]` to `Fd`
