// A `T` an associated type is equal to can come from the iterator, not
// only from the arguments of type `T`.
fn next_or<T: Copy, I: Iterator<Item = T> + Discard>(it: I, d: T) -> T {
    let mut it = it;
    it.next().unwrap_or(d)
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[next_or(9..10usize, i)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
