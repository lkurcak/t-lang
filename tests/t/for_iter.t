// `for` over any `Iterator`: the prelude's trait, which a program's own
// types can implement. Its provided methods build on `next`.

#[derive(Discard)]
struct Countdown { n: int }

impl Iterator for Countdown {
    type Item = int;

    fn next(&mut self) -> Option<int> {
        if self.n == 0 {
            return Option::None;
        }
        self.n = self.n - 1;
        Option::Some(self.n + 1)
    }
}

// Items needn't be `Discard`: the body uses each up.
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
        if self.left == 0 {
            return Option::None;
        }
        self.left = self.left - 1;
        Option::Some(Fd { n: self.left })
    }
}

fn total<I: Iterator<Item = int> + Discard>(it: I) -> int {
    it.fold(0, |acc, x| acc + x)
}

fn main() {
    for i in (Countdown { n: 3 }) {
        print(i);
    }
    // expect: 3
    // expect: 2
    // expect: 1

    print(total(0..5)); // expect: 10
    print(total((0..5).map(|x| x * x))); // expect: 30
    print((0..10).map(|x| x * 2).count()); // expect: 10
    print(total((Countdown { n: 4 }).map(|x| x - 1))); // expect: 6

    let mut closed = 0;
    for fd in (Fds { left: 3 }) {
        closed = closed + close(fd);
    }
    print(closed); // expect: 3

    // `for x in &mut it` only borrows `it`: what's left of it is still
    // there after a `break`.
    let mut it = 0..10;
    for i in &mut it {
        if i == 3 {
            break;
        }
    }
    print(it.next().unwrap_or(-1)); // expect: 4
    print(total(it)); // expect: 35
}
