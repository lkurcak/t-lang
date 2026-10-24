// Taking a value apart moves it, so it can't be borrowed then.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let fd = open(1);
    let r = &fd;
    let Fd { n } = fd;
    out.line(&(r.n + n))?;
    Result::Ok(())
}
// error: cannot move out of `fd` while `fd` is borrowed
