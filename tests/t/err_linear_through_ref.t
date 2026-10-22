// What's behind a `&mut` can't be thrown away either: `replace` gives it back.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn set(r: &mut Fd) {
    *r = open(2);
}

fn main() {
    let mut fd = open(1);
    set(&mut fd);
    print(close(fd));
}
// error: use `core::mem::replace(&mut *r, ..)`
