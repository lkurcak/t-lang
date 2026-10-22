// A value whose type isn't `Discard` can't just go out of scope, even if its
// name starts with `_`.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let _fd = open(1);
}
// error: `_fd` is never used up, and its type, `Fd`, isn't `Discard`
