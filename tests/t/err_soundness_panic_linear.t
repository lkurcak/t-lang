// A panic aborts, so the linear value it leaves behind is no leak: the
// interpreter doesn't count what's dropped on the way out of one.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

fn main() with panic {
    let fd = Fd { n: 1 };
    let z: i64 = 0;
    let _v = 1 / z;
    let _v = close(fd);
}
// error: division by zero
