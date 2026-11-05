// `I: Iterator<Item = T>` decides `T`: it's what `I`'s values are.
#[derive(Discard where I: Discard)]
struct Skip1<I> { iter: I, skipped: bool }

impl<T: Discard, I: Iterator<Item = T>> Iterator for Skip1<I> {
    type Item = T;
    effect e = I::e;

    fn next(&mut self) -> Option<T> with I::e {
        if !self.skipped {
            self.skipped = true;
            match self.iter.next() {
                Some(_x) => {},
                None => return None,
            }
        }
        self.iter.next()
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    for x in (Skip1 { iter: 0..4, skipped: false }).map(|x| x * 10i64) {
        out.line(&x)?;
    }
    Ok(())
    // expect: 10
    // expect: 20
    // expect: 30
}
