// Of a type parameter, `!` needs to know it's an integer type.
fn f<T: Copy>(x: T) -> T { !x }

fn main() {}
// error: `!` works on `bool` and integers, not on `T`; add a bound: `T: Int`
