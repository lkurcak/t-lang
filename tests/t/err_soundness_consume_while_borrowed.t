// A pattern that takes a linear value apart moves all of it, so not while
// part of it is borrowed.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

struct P { a: Fd, b: Fd }

fn main() with panic {
    let p = P { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let r = &p.b;
    let P { a, b } = p;
    print(r.n);
    print(close(a) + close(b));
}
// error: cannot move out of `p` while `p.b` is borrowed
