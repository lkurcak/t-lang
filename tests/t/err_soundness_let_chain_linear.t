// Where the second condition doesn't hold, `fd` is still there, and
// nothing uses it up.
struct Fd { n: u8 }

fn close(fd: Fd) -> u8 {
    let Fd { n } = fd;
    n
}

fn f(o: Option<Fd>, c: bool) -> u8 {
    if let Some(fd) = o && c { close(fd) } else { 0 }
}

fn main() {
    let _ = f(Some(Fd { n: 1 }), false);
}
// error: `fd`
