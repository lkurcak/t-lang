// Used up on one path only.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = open(1);
    if 1i64 < 2 {
        out.line(&close(fd))?;
    }
    Result::Ok(())
}
// error: `fd` isn't used up on every path
