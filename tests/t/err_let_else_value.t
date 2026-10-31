// An `else` that gives a value finishes too.
fn f(o: Option<i64>) -> i64 {
    let Option::Some(n) = o else { 0 };
    n
}

fn main() {
    let _ = f(Option::None);
}
// error: the `else` of a `let .. else` must not finish
