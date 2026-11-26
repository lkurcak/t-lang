// The same where a function is named.
fn invariant(x: u8) -> u8 {
    x
}

fn main() {
    let _ = invariant(1);
}
// error: `invariant` is a keyword, so it can't be a name
