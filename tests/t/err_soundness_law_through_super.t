// Only where `Self` implements the law's trait is the law known: through
// `I: Iterator` alone, `next` may give `None` and then a value, which
// `ExactSizeIterator`'s law of `next` would rule out
// (docs/88-iterator-laws.md).
fn fused<I: Iterator>(it: &mut I) -> { b: bool | b } with I::e where I::Item: Discard {
    let a = it.next();
    let b = it.next();
    a.is_some() || b.is_none()
}

// `None`, then a value, then `None`, ..
#[derive(Discard)]
struct Flip { on: bool }

impl Iterator for Flip {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> {
        self.on = !self.on;
        if self.on { Some(1) } else { None }
    }
}

fn main() {
    let _ = fused(&mut Flip { on: true });
}
// error: can't prove `b` for the result of `fused`
