// `_` binds nothing, so nothing uses the value up.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let _ = open(1);
}
// error: this value is never used up
