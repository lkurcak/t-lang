// Taking a value apart moves it, so it can't be borrowed then.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() with panic {
    let fd = open(1);
    let r = &fd;
    let Fd { n } = fd;
    print(r.n + n);
}
// error: cannot move out of `fd` while `fd` is borrowed
