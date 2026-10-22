// A box is `Discard` only if what it holds is.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() with {alloc, panic} {
    let _b = Box::new(open(1));
}
// error: `_b` is never used up, and its type, `Box<Fd>`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Fd`
