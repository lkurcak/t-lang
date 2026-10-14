// Together they match every int.
fn main() {
    let ..0 | 0.. = 5;
}
// error: `..0` and `0..` can be merged into `_`
