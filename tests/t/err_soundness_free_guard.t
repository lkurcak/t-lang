// An `Ok`'s payload is what a function gives only if the value was an
// `Ok`: `either` gives the `Err`'s, of the same type.
fn either<T>(r: Result<T, T>) -> T {
    match r {
        Ok(x) => x,
        Err(e) => e,
    }
}

fn sorted(n: usize) -> Result<{ v: usize | v < 4 }, usize> {
    if n < 4 { Ok(n) } else { Err(n) }
}

fn at(a: [u8; 4], n: usize) -> u8 { a[either(sorted(n))] }

fn main() {}
// error: can't prove `i < self.len()` for argument `i` of `index`
