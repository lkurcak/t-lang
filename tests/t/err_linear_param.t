// A parameter that isn't used up is dropped when the function returns.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn ignore(_fd: Fd) -> int { 0 }

fn main() {
    print(ignore(open(1)));
}
// error: `_fd` is dropped here, when the function returns
