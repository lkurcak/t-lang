// `_` inside a pattern that takes the value apart throws that part away.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

enum Either { Left(Fd), Right(Fd) }

fn main() {
    let e = Either::Left(open(1));
    match e {
        Either::Left(_) | Either::Right(_) => {},
    }
}
// error: this pattern throws away `e.0`
