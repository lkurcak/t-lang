// A `let` is joined to the rest of a condition by `&&` only: with `||`,
// what it binds would be there on one side and not the other.
fn f(o: Option<u8>, c: bool) -> u8 {
    if let Some(a) = o || c { a } else { 0 }
}

fn main() {
    let _ = f(None, true);
}
// error: `||`
