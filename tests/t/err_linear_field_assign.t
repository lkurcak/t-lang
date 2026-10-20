// Giving a field a new value throws its old one away.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

struct Pair { a: Fd, b: Fd }

fn main() with panic {
    let mut p = Pair { a: open(1), b: open(2) };
    p.a = open(3);
    print(close(p.a) + close(p.b));
}
// error: this assignment would throw away the value `p.a` holds
