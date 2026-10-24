// A closure that owns a value that must be used up is called once, and
// uses it up.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn later<F: FnOnce() -> i64>(f: F) -> i64 { f() }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let fd = Fd { n: 3 };
    let c = || close(fd);
    out.line(&later(c))?;                // expect: 3
    let fd = Fd { n: 4 };
    let c = move || close(fd);
    out.line(&c())?;                     // expect: 4
    Result::Ok(())
}
