// Together they match every int.
fn main() {
    let ..0 | 0.. = 5;
}
// warning: `..0` and `0..` can be merged into `_`
