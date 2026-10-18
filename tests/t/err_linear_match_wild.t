// A `_` arm doesn't take the value apart, so it isn't used up there.
struct Fd { n: int }

fn open(n: int) -> Fd { Fd { n } }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

#[derive(Discard)]
enum Option<T> { Some(T), None }

fn main() {
    let o = Option::Some(open(1));
    match o {
        Option::Some(fd) => print(close(fd)),
        _ => {},
    }
}
// error: `o` isn't used up on every path
