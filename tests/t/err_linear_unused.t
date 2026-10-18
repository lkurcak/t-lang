// A value whose type isn't `Discard` can't just go out of scope, even if its
// name starts with `_`.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let _fd = open(1);
}
// error: `_fd` is never used up, and its type, `Fd`, isn't `Discard`
