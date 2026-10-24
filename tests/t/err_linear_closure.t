// A closure's parameters are used up like a function's.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let f = |_fd: Fd| 1i64;
    out.line(&f(open(1)))?;
    Result::Ok(())
}
// error: `_fd` is never used up
