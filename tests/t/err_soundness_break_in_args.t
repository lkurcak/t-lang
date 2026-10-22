// The same for `break`.
struct Fd { n: i64 }

fn close(f: Fd) -> i64 {
    let Fd { n } = f;
    n
}

fn two(a: Fd, b: i64) -> i64 with panic { close(a) + b }

fn main() with panic {
    let x = loop {
        two(Fd { n: 1 }, break 3i64);
    };
    print(x);
}
// error: this value is never used up, and its type, `Fd`, isn't `Discard`
