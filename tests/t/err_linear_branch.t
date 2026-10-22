// Used up on one path only.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let fd = open(1);
    if 1i64 < 2 {
        print(close(fd));
    }
}
// error: `fd` isn't used up on every path
