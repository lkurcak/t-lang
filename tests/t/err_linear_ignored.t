// An ignored result is a temporary, dropped at the end of its statement.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    open(1);
}
// error: this value is never used up, and its type, `Fd`, isn't `Discard`
