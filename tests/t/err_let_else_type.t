// A `let .. else` has no type of its own: its pattern says it, with type
// arguments if it needs them.
fn f(o: Option<i64>) -> i64 {
    let Option::Some(n): Option<i64> = o else { return 0 };
    n
}

fn main() {
    let _ = f(Option::None);
}
// error: a `let .. else` can't have a type
