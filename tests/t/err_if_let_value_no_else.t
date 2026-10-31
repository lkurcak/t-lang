// With no `else`, there's nothing to be when the pattern doesn't match.
fn f(o: Option<i64>) -> i64 {
    if let Option::Some(n) = o { n }
}

fn main() {
    let _ = f(Option::None);
}
// error: `if let` without `else` must have type `()`, found `i64`
