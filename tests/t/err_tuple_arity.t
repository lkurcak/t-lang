// A pattern for three values doesn't match a pair.
fn main() {
    let (_a, _b, _c) = (1u8, 2u8);
}
// error: this pattern is for a tuple of 3 values, but the value is `(u8, u8)`
