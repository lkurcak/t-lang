// `_` inside a pattern that takes the value apart throws that part away.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
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
