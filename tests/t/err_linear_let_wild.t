// `_` binds nothing, so nothing uses the value up.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let _ = open(1);
}
// error: this value is never used up
