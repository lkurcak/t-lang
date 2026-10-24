// A type parameter and an effect parameter share one list of names.
fn f<T, effect T>(x: T) {}

fn main() {}
// error: type parameter `T` is declared twice
// warning: effect parameter `T` should have a snake case name: `t`
