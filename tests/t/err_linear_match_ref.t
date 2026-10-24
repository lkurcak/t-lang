// Matching through a reference only looks: the value is still there.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = open(1);
    match &fd {
        Fd { n } => out.line(&(*n))?,
    }
    Result::Ok(())
}
// error: `fd` is never used up
