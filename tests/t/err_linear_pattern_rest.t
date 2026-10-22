// `..` throws away the fields it skips.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

struct Pair { a: Fd, b: Fd }

fn main() {
    let Pair { a, .. } = Pair { a: open(1), b: open(2) };
    print(close(a));
}
// error: this pattern throws away part of the value it matches, and its type, `Fd`, isn't `Discard`
