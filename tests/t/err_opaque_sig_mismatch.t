// The closure returns an `i64`, not the `bool` the return type promises.
fn f() -> impl Fn(i64) -> bool + Discard { |x| x + 1 }

fn main() { print_bool(f()(1)); }
// error: expected `bool`, found `i64`
