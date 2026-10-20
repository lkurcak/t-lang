// `continue` skips the rest of the body, which would use up the item.
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

#[derive(Discard)]
struct Fds { left: int }

impl Iterator for Fds {
    type Item = Fd;

    fn next(&mut self) -> Option<Fd> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

fn main() {
    for fd in (Fds { left: 2 }) {
        if fd.n == 1 {
            continue;
        }
        print(close(fd));
    }
}
// error: `fd` is never used up
