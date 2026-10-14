enum Option<T> { Some(T), None }
fn is_none<T>(o: Option<T>) -> bool { o == Option::None }
fn main() { print_bool(is_none(Option::Some(1))); }
// error: cannot compare `Option<T>` with `==`: it contains `T`, a type parameter
