// A closure the function is given can make a `T`.
fn make_with<T: Copy, F: Fn() -> T + Discard>(d: T, f: F) -> T {
    let _ = d;
    f()
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[make_with(i, || 9usize)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
