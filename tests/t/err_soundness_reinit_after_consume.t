// After a pattern has used `p` up, there's no `p.a` to give a value to.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

struct P { a: Fd, b: Fd }

fn main() with panic {
    let mut p = P { a: Fd { n: 1 }, b: Fd { n: 2 } };
    let P { a, b } = p;
    p.a = Fd { n: 3 };
    print(close(a) + close(b));
}
// error: cannot assign to `p.a`, part of `p`, which was moved out of
