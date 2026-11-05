// What the pattern binds isn't there in the `else`: it didn't match.
fn f(o: Option<i64>) -> i64 {
    let Some(n) = o else { return n };
    n
}

fn main() {
    let _ = f(None);
}
// error: unknown variable `n`
