// A pattern that moves some of an array's values out leaves the rest in
// it, still to be used up.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let fds = [Fd { n: 1 }, Fd { n: 2 }];
    let [a, _] = fds;
    close(a);
}
// error: `fds[1]` is never used up, and its type, `Fd`, isn't `Discard`
