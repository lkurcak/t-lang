// One `pop` empties a list of one, but not of more.
use alloc::list::List;

struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn one(mut l: List<Fd>) -> i64 {
    match l.pop() {
        Some(fd) => close(fd),
        None => 0,
    }
}

fn main() with alloc {
    let mut l: List<Fd> = List::new();
    l.push(Fd { n: 1 });
    l.push(Fd { n: 2 });
    let _ = one(l);
}
// error: `l` is dropped here, and `List<Fd>` isn't `Discard`: only `{ l: List<Fd> | l.len() == 0 }` is, which can't be proved of it
