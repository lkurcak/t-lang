// Used up on one path only.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let fd = open(1);
    if 1 < 2 {
        print(close(fd));
    }
}
// error: `fd` isn't used up on every path
