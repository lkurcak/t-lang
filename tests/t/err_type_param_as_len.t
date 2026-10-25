// An array's length is a `usize`, not a type.
fn f<T>(a: [i64; T]) {}

fn main() {}
// error: `T` is a type parameter, not a const; declare it as `const T: usize`
