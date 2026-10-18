// Moving one field out leaves the other to use up.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

struct Pair { a: Fd, b: Fd }

fn main() {
    let p = Pair { a: open(1), b: open(2) };
    let a = p.a;
    print(close(a));
}
// error: `p.b` is never used up
