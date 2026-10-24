// A variable declared with no value can be given one on each path; giving
// it a second throws the first away.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let x;
    if true {
        x = Fd { n: 1 };
    } else {
        x = Fd { n: 2 };
    }
    out.line(&close(x))?;
    let mut y;
    y = Fd { n: 3 };
    y = Fd { n: 4 };
    out.line(&close(y))?;
    Result::Ok(())
}
// error: this assignment would throw away the value `y` holds, and its type, `Fd`, isn't `Discard`
