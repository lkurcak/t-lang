// A parameter that isn't used up is dropped when the function returns.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn ignore(_fd: Fd) -> i64 { 0 }

fn main() {
    print(ignore(open(1)));
}
// error: `_fd` is dropped here, when the function returns
