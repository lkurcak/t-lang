// A derived `Eq` needs `Eq` of what's in it: `Option<T>` is `Eq` if `T` is.
#[derive(Eq)]
enum Option<T> { Some(T), None }
fn is_none<T>(o: Option<T>) -> bool { o == Option::None }
fn main() { print_bool(is_none(Option::Some(1))); }
// error: cannot compare `Option<T>` with `==`: that needs `T: Eq`; add a bound: `T: Eq`
