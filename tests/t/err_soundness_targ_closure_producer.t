// Nor one given to a function that can make a `T`.
use alloc::list::List;

trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 9 }
}

fn feed<T: Make, F: FnMut(T) + Discard>(l: &List<T>, mut f: F) {
    let _ = l;
    f(T::make());
}

fn main() with alloc {
    let a: [u8; 4] = [1, 2, 3, 4];
    let q: List<{ t: usize | t < 4 }> = List::new();
    feed(&q, |t| {
        let _ = a[t];
    });
}
// error: can't prove `i < self.len()` for argument `i` of `index`
