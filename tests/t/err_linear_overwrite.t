// Giving a variable a new value throws the old one away.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut fd = open(1);
    fd = open(2);
    out.line(&close(fd))?;
    Result::Ok(())
}
// error: this assignment would throw away the value `fd` holds, and its type, `Fd`, isn't `Discard`
