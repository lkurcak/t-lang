// A closure that owns a field that must be used up is `FnOnce`, and uses
// it up; the variable's other fields are its own to use up, not the
// closure's.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

struct Two { a: Fd, b: Fd }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let t = Two { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let first = move || close(t.a);
    let b = close(t.b);
    out.line(&(first() * 10 + b)) // expect: 12
}
