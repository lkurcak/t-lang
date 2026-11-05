// `filter` throws away what it doesn't keep, so the values must be
// `Discard`.
struct Fd { n: i64 }

#[derive(Discard)]
struct Fds { left: i64 }

impl Iterator for Fds {
    type Item = Fd;
    effect e = {};

    fn next(&mut self) -> Option<Fd> {
        if self.left == 0 {
            return None;
        }
        self.left = self.left - 1;
        Some(Fd { n: self.left })
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let kept = (Fds { left: 3 }).filter(|fd| fd.n > 0);
    for Fd { n } in kept {
        out.line(&n)?;
    }
    Ok(())
}
// error: `filter` needs its `Self::Item` to implement `Discard`, and `Fd` doesn't
