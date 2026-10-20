// A `break` leaves the loop with the item it's on, which must be used up
// first.
struct Fd { n: int }

fn close(fd: Fd) -> int {
    let Fd { n } = fd;
    n
}

#[derive(Discard)]
struct Fds { left: int }

impl Iterator for Fds {
    type Item = Fd;
    effect e = {};

    fn next(&mut self) -> Option<Fd> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

fn main() with panic {
    let mut total = 0;
    for fd in (Fds { left: 3 }) {
        if fd.n == 1 {
            break;
        }
        total = total + close(fd);
    }
    print(total);
}
// error: `fd` is never used up
