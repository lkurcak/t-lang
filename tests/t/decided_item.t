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
                Option::Some(_x) => {},
                Option::None => return Option::None,
            }
        }
        self.iter.next()
    }
}

fn main() with panic {
    for x in (Skip1 { iter: 0..4, skipped: false }).map(|x| x * 10) {
        print(x);
    }
    // expect: 10
    // expect: 20
    // expect: 30
}
