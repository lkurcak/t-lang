// An iterator that gives what it holds, but whose `DoubleEndedIterator`
// impl can make one.
trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 100 }
}

// Gives the one value it holds; its DoubleEndedIterator impl makes one.
#[derive(Discard)]
struct Once<T> { v: Option<T> }

impl<T: Discard> Iterator for Once<T> {
    type Item = T;
    effect e = {};

    fn next(&mut self) -> Option<T> { core::mem::replace(&mut self.v, None) }
}

impl<T: Discard + Make> DoubleEndedIterator for Once<T> {
    fn next_back(&mut self) -> Option<T> {
        let _ = core::mem::replace(&mut self.v, None);
        Some(T::make())
    }
}

fn main() {
    let a: [u8; 8] = [0; 8];
    let it = Once { v: Some(3usize) };
    for r in it.rev() {
        let _ = a[r];
        break;
    }
}
// error: can't prove `i < self.len()`
