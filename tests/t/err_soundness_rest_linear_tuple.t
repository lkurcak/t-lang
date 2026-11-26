// The same of a tuple's values.
struct Fd { n: u8 }

fn close(fd: Fd) -> u8 {
    let Fd { n } = fd;
    n
}

fn f(t: (Fd, Fd)) -> u8 {
    let (a, ..) = t;
    close(a)
}

fn main() {
    let _ = f((Fd { n: 1 }, Fd { n: 2 }));
}
// error: `t.1` is never used up
