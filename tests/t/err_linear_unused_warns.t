// An unused variable whose type isn't `Discard` is warned about, but not
// with a fix to rename it `_fd`: it would still have to be used up.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = open(1); // warning: unused variable `fd`: use it up, as its type, `Fd`, isn't `Discard`
    let n: i64 = 2; // warning: unused variable `n`
    out.line(&close(open(3)))?;
    Result::Ok(())
}
// error: `fd` is never used up
