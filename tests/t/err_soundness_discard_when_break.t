// A `for` that stops early, by `break`, throws its iterator away with
// values still in it: it's `Discard` only when it's used up (docs/80).
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    for fd in l {
        if close(fd) == 1 {
            break;
        }
    }
}
// error: the `for` may stop before its iterator is used up, and drops it then: `IntoIter<Fd>` isn't `Discard`, only `{ it: IntoIter<Fd> | it.len() == 0 }` is
