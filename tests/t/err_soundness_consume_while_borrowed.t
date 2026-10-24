// A pattern that takes a linear value apart moves all of it, so not while
// part of it is borrowed.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

struct P { a: Fd, b: Fd }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let p = P { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let r = &p.b;
    let P { a, b } = p;
    out.line(&r.n)?;
    out.line(&(close(a) + close(b)))?;
    Result::Ok(())
}
// error: cannot move out of `p` while `p.b` is borrowed
