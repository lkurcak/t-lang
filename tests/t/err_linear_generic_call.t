// With the bound, it's the caller that has to show it holds.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn forget<T: Discard>(_x: T) {}

fn main() {
    forget(open(1));
}
// error: `forget` needs its `T` to implement `Discard`, and `Fd` doesn't
