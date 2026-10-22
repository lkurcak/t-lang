// Together they match every i64.
fn main() {
    let ..0 | 0.. = 5i64;
}
// warning: `..0` and `0..` can be merged into `_`
