// Chains of `let`s and conditions aren't there (yet): `&&` would be part of
// the expression matched, which is a `bool`.
fn f(o: Option<i64>, c: bool) -> i64 {
    if let Option::Some(n) = o && c { n } else { 0 }
}

fn main() {
    let _ = f(Option::None, true);
}
// error: `&&` can't come at the top of what an `if let` matches
