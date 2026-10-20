// A provided method with a `where` clause can only be called where it
// holds: `count` throws away the iterator, and its items.
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
    print(Fds { left: 3 }.count());
}
// error: `count` needs its `Self::Item` to implement `Discard`, and `Fd` doesn't
