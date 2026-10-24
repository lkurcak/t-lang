// Dropping a list drops what's in it, so it's `Discard` only if its values
// are; a list of linear ones is used up by `free_with`.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with {alloc, panic} {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.discard();
}
// error: `List<Fd>` doesn't implement `Discard`, which method `discard` is of; that needs `Fd: Discard`; add `#[derive(Discard)]` to `Fd`
