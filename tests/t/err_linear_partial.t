// Moving one field out leaves the other to use up.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

struct Pair { a: Fd, b: Fd }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let p = Pair { a: open(1), b: open(2) };
    let a = p.a;
    out.line(&close(a))?;
    Result::Ok(())
}
// error: `p.b` is never used up
