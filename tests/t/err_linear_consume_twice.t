// Once it's been taken apart, it's gone: it can't be used up twice.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = open(1);
    let Fd { n } = fd;
    out.line(&n)?;
    out.line(&close(fd))?;
    Result::Ok(())
}
// error: use of moved value `fd`
