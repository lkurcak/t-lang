#[derive(Discard)]
enum Option<T> { Some(T), None }
fn f(o: Option) {}
fn main() {}
// error: `Option` takes 1 type argument but 0 were supplied
