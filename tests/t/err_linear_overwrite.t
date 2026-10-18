// Giving a variable a new value throws the old one away.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let mut fd = open(1);
    fd = open(2);
    print(close(fd));
}
// error: this assignment would throw away the value `fd` holds, and its type, `Fd`, isn't `Discard`
