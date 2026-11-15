// So does one that returns from inside.
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn first(l: List<Fd>) -> i64 {
    for fd in l {
        return close(fd);
    }
    0
}

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    let _ = first(l);
}
// error: the `for` may stop before its iterator is used up, and drops it then: `IntoIter<Fd>` isn't `Discard`, only `{ it: IntoIter<Fd> | it.len() == 0 }` is
