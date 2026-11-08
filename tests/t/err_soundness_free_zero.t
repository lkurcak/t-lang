// With no argument to come from, a `Some` can't be there, but only of a
// type parameter that's free: a bound that can make one can fill it.
trait Make {
    fn make() -> Self;
}

impl Make for usize {
    fn make() -> usize { 7 }
}

fn made<T: Make>() -> Option<T> { Some(T::make()) }

fn at(a: [u8; 4]) -> u8 {
    match made::<usize>() {
        Some(i) => a[i],
        None => 0,
    }
}

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
