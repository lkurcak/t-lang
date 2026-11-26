// What a chain's `let` binds isn't there in its `else`.
fn f(o: Option<u8>) -> u8 {
    if let Some(a) = o && a > 3 { a } else { a }
}

fn main() {
    let _ = f(None);
}
// error: unknown variable `a`
