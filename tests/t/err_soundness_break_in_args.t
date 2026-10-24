// The same for `break`.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

fn two(a: Fd, b: i64) -> i64 with panic { close(a) + b }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let x = loop {
        two(Fd { n: 1 }, break 3i64);
    };
    out.line(&x)?;
    Result::Ok(())
}
// error: this value is never used up, and its type, `Fd`, isn't `Discard`
