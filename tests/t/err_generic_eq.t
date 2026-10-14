// `T` may be a function, so `==` on it needs a bound (traits, step 6).
fn same<T>(a: T, b: T) -> bool { a == b }
fn main() { print_bool(same(1, 1)); }
// error: cannot compare values of type `T` with `==`
