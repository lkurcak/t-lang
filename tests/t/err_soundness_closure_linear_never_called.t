// A closure that owns a value that must be used up must be called.
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() {
    let fd = Fd { n: 3 };
    let c = || close(fd);
}
// warning: unused variable `c`: use it up
// error: `c` is never used up
