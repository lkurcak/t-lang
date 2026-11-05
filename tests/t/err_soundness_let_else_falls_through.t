// If the `else` could finish, the code after it would run with `n` never
// given a value.
fn f(o: Option<i64>) -> i64 {
    let Some(n) = o else {};
    n
}

fn main() {
    let _ = f(None);
}
// error: the `else` of a `let .. else` must not finish
