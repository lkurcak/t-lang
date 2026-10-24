// The closure gives away the `Fd` it owns, so it can only be called once,
// but the return type says it can be called any number of times.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn once(fd: Fd) -> impl Fn() -> i64 { move || close(fd) }

fn main(os: std::Os) -> Result<(), std::io::Error> { let mut out = os.stdout; out.line(&once(Fd { n: 1 })())?;  Result::Ok(()) }
// error: this closure is only `FnOnce`, as
