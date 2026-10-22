// An unused variable whose type isn't `Discard` is warned about, but not
// with a fix to rename it `_fd`: it would still have to be used up.
struct Fd { n: i64 }

fn open(n: i64) -> Fd { Fd { n } }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() {
    let fd = open(1); // warning: unused variable `fd`: use it up, as its type, `Fd`, isn't `Discard`
    let n: i64 = 2; // warning: unused variable `n`
    print(close(open(3)));
}
// error: `fd` is never used up
