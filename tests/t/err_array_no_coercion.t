// A reference to an array isn't a slice: `as_slice` makes one.
fn total(s: &[i64]) -> usize { s.len() }

fn main() {
    let a = [1, 2, 3];
    total(&a);
}
// error: expected `[i64]`, found `[{integer}; 3]`
