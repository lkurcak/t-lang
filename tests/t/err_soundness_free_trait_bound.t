// A trait whose method makes a `Self` can make a `T` from nothing: a type
// parameter it bounds has no free theorem, even where it's also `Copy`.
trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 7 }
}

fn remake<T: Make + Copy>(x: T) -> T {
    let _ = x;
    T::make()
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[remake(i)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
