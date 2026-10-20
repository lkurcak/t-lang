// `filter` throws away what it doesn't keep, so the values must be
// `Discard`.
struct Fd { n: int }

#[derive(Discard)]
struct Fds { left: int }

impl Iterator for Fds {
    type Item = Fd;

    fn next(&mut self) -> Option<Fd> {
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

fn main() {
    let kept = (Fds { left: 3 }).filter(|fd| fd.n > 0);
    for Fd { n } in kept {
        print(n);
    }
}
// error: `filter` needs its `Self::Item` to implement `Discard`, and `Fd` doesn't
