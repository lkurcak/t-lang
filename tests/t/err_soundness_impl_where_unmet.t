// What an impl's `where` clause needs, it needs where the impl is used:
// `Second` throws away every other value, and an `Fd` can't be.
struct Fd { n: i64 }

#[derive(Discard)]
struct Fds { left: i64 }

impl Iterator for Fds {
    type Item = Fd;
    effect e = panic;

    fn next(&mut self) -> Option<Fd> with panic {
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

#[derive(Discard where I: Discard)]
struct Second<I> { iter: I }

impl<I: Iterator> Iterator for Second<I> where I::Item: Discard {
    type Item = I::Item;
    effect e = I::e;

    fn next(&mut self) -> Option<I::Item> with I::e {
        match self.iter.next() {
            Option::Some(_x) => self.iter.next(),
            Option::None => Option::None,
        }
    }
}

fn main() with panic {
    for Fd { n } in (Second { iter: Fds { left: 4 } }) {
        print(n);
    }
}
// error: `Second<Fds>` doesn't implement `Iterator`, which method `next` is of; that needs `Fd: Discard`
