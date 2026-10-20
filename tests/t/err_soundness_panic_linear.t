// A panic aborts, so the linear value it leaves behind is no leak: the
// interpreter doesn't count what's dropped on the way out of one.
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

fn main() with panic {
    let fd = Fd { n: 1 };
    let z = 0;
    print(1 / z);
    print(close(fd));
}
// error: division by zero
