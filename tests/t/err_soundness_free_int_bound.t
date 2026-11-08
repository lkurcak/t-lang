// A `T: Int` can be made from a literal, so what a function generic in it
// gives needn't be what it was given: no free theorem
// (docs/55-free-theorems.md).
fn nine<T: Int>(x: T) -> T {
    let _ = x;
    9
}

fn at(a: [u8; 4], i: { v: usize | v < 4 }) -> u8 { a[nine(i)] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
