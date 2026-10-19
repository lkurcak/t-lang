// The closure returns an `int`, not the `bool` the return type promises.
fn f() -> impl Fn(int) -> bool + Discard { |x| x + 1 }

fn main() { print_bool(f()(1)); }
// error: expected `bool`, found `int`
