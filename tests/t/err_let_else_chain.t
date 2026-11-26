// `let .. else` takes one pattern and one value, not a chain.
fn f(o: Option<u8>, c: bool) -> u8 {
    let Some(a) = o && c else { return 0 };
    a
}

fn main() {
    let _ = f(None, true);
}
// error: `&&` can't come at the top of what a `let .. else` matches
