// The closure owns an `Fd`, which must be used up, so it isn't `Discard`,
// but the return type says it is: a caller could drop it, and the `Fd`.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn once(fd: Fd) -> impl FnOnce() -> i64 + Discard { move || close(fd) }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&once(Fd { n: 1 })())?;  Result::Ok(()) }
// error: doesn't implement `Discard`, which `once`'s return type says
