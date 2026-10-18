// A closure's parameters are used up like a function's.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let f = |_fd: Fd| 1;
    print(f(open(1)));
}
// error: `_fd` is never used up
