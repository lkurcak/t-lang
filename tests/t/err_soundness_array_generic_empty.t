// `N` can be 0, so a generic array may have no first value.
fn first<const N: usize>(a: &[i64; N]) -> i64 { a[0] }

fn main() {
    let e: [i64; 0] = [];
    first(&e);
}
// error: can't prove `i < self.len()` for argument `i` of `index`
