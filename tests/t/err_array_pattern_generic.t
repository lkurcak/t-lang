// `N` could be any length, so `[x]` can't match an array of `N`.
fn one<const N: usize>(a: [i64; N]) -> i64 {
    let [x] = a;
    x
}

fn main() {}
// error: this pattern is for an array of 1, but the value is `[i64; N]`
