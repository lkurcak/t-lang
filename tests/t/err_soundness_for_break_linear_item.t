// A `break` leaves the loop with the item it's on, which must be used up
// first.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

#[derive(Discard)]
struct Fds { left: i64 }

impl Iterator for Fds {
    type Item = Fd;
    effect e = {};

    fn next(&mut self) -> Option<Fd> {
        if self.left == 0 { return Option::None; }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let mut total = 0;
    for fd in (Fds { left: 3 }) {
        if fd.n == 1 {
            break;
        }
        total = total + close(fd);
    }
    out.line(&total)?;
    Result::Ok(())
}
// error: `fd` is never used up
