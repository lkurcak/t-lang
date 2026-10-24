// `truncate` throws values away, so they must be `Discard`.
use alloc::list::List;

struct Fd { n: i64 }

fn main() with {alloc, panic} {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.truncate(0);
    l.free_with(|fd: Fd| {
        let Fd { n } = fd;
        let _v = n;
    });
}
// error: `List<T>::truncate` needs its `T` to implement `Discard`, and `Fd` doesn't; add `#[derive(Discard)]` to `Fd`
