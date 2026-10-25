// An array of three has no value at index 3.
fn main() {
    let a = [1i64, 2, 3];
    let _x = a[3];
}
// error: can't prove `i < self.len()` for argument `i` of `index`
