// Stricter than it needs to be: `n` is borrowed through `p.q.r`, a shared
// reference, and using `p` up doesn't touch what that points to. Rust
// accepts this; the borrow checker here treats moving `p` as reaching
// through the references in it.
struct Fd { n: int }

#[derive(Discard)]
struct Q<'a> { r: &'a Fd }

struct P<'a> { a: Fd, q: Q<'a> }

fn close(f: Fd) -> int {
    let Fd { n } = f;
    n
}

fn main() with panic {
    let inner = Fd { n: 9 };
    let p = P { a: Fd { n: 1 }, q: Q { r: &inner } };
    let P { a, q: Q { r: Fd { n } } } = p;
    print(close(a) + *n);
    print(close(inner));
}
// error: cannot move out of `p` while `p.q.r.n` is borrowed
