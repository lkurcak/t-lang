// `for` over any `Iterator`: the prelude's trait, which a program's own
// types can implement. Its provided methods build on `next`.

#[derive(Discard)]
struct Countdown { n: i64 }

impl Iterator for Countdown {
    type Item = i64;
    effect e = panic;

    fn next(&mut self) -> Option<i64> with panic {
        if self.n == 0 {
            return Option::None;
        }
        self.n = self.n - 1;
        Option::Some(self.n + 1)
    }
}

// Items needn't be `Discard`: the body uses each up.
struct Fd { n: i64 }

fn close(fd: Fd) -> i64 {
    let Fd { n } = fd;
    n
}

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

fn total<I: Iterator<Item = i64> + Discard>(it: I) -> i64 with {panic, I::e} {
    it.fold(0, |acc, x| acc + x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    for i in (Countdown { n: 3 }) {
        out.line(&i)?;
    }
    // expect: 3
    // expect: 2
    // expect: 1

    out.line(&total(0..5))?; // expect: 10
    out.line(&total((0..5).map(|x| x * x)))?; // expect: 30
    out.line(&(0..10).map(|x| x * 2i64).count())?; // expect: 10
    out.line(&total((Countdown { n: 4 }).map(|x| x - 1)))?; // expect: 6

    let mut closed = 0;
    for fd in (Fds { left: 3 }) {
        closed = closed + close(fd);
    }
    out.line(&closed)?; // expect: 3

    // `for x in &mut it` only borrows `it`: what's left of it is still
    // there after a `break`.
    let mut it = 0..10;
    for i in &mut it {
        if i == 3 {
            break;
        }
    }
    out.line(&it.next().unwrap_or(-1))?; // expect: 4
    out.line(&total(it))?; // expect: 35
    Result::Ok(())
}
