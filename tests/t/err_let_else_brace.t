// Which `else` would this be? As in Rust, what's before a `let`'s `else`
// can't end in `}`.
fn f(c: bool, o: Option<i64>) -> i64 {
    let n = if c { 1 } else { 2 } else { return 0 };
    n
}

fn main() {
    let _ = f(true, Option::None);
}
// error: what's before the `else` of a `let .. else` can't end in `}`
