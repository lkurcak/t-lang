// `N` could be any length, even 0, so `[x, ..]` can't match an array of
// `N`: an array's pattern needs its length known (lemma A6 of
// docs/22-arrays.md).
fn first<const N: usize>(a: [u8; N]) -> u8 {
    let [x, ..] = a;
    x
}

fn main() {}
// error: this pattern is for an array of at least 1, but the value is `[u8; N]`
