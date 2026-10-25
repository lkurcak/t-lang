// `[a, b]` matches arrays of two, not three.
fn main() {
    let [_a, _b] = [1, 2, 3];
}
// error: this pattern is for an array of 2, but the value is `[{integer}; 3]`
