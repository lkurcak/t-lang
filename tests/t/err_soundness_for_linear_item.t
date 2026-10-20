// Each item a `for` binds must be used up by its body, like any variable.
struct Fd { n: int }

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
        print(fd.n);
    }
}
// error: `fd` is never used up, and its type, `Fd`, isn't `Discard`
