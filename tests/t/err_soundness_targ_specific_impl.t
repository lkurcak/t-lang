// An impl of `Iterator` for one type, `Wrap<usize>`, can change its value
// as it likes: generic code bounded `Iterator` that's given a refined
// `Wrap` can't keep its refinement. (One generic in all of `Wrap`'s type
// arguments, as `Iter`'s is, can only keep and give back what's there.)
#[derive(Discard)]
struct Wrap<T> { x: T }

impl Iterator for Wrap<usize> {
    type Item = u8;
    effect e = {};

    fn next(&mut self) -> Option<u8> {
        self.x = 99;
        None
    }
}

fn drain<I: Iterator<Item: Discard>>(it: &mut I) {
    let _ = it.next();
}

fn main() {
    let mut w: Wrap<{ t: usize | t < 4 }> = Wrap { x: 1 };
    drain(&mut w);
}
// error: can make or change a value of it
