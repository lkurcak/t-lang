// The old value is still there the next time round.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let mut fd = open(0);
    let mut i = 0;
    while i < 3 {
        fd = open(i);
        i += 1;
    }
    print(close(fd));
}
// error: this assignment would throw away the value `fd` holds
