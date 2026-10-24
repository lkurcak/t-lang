// So does `break`.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    loop {
        let fd = open(1);
        if 1i64 < 2 {
            break;
        }
        out.line(&close(fd))?;
    }
    Result::Ok(())
}
// error: `fd` is never used up
