// The same for `break`.
struct Fd { n: int }

fn close(f: Fd) -> int {
    let Fd { n } = f;
    n
}

fn two(a: Fd, b: int) -> int with panic { close(a) + b }

fn main() with panic {
    let x = loop {
        two(Fd { n: 1 }, break 3);
    };
    print(x);
}
// error: this value is never used up, and its type, `Fd`, isn't `Discard`
