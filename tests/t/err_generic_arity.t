#[derive(Discard)]
enum Option<T> { Some(T), None }
fn f(o: Option<i64, bool>) {}
fn main() {}
// error: `Option` takes 1 type argument but 2 were supplied
