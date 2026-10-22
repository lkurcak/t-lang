// In generic code a literal must fit every integer type: 0 to 127.
fn f<T: Int>() -> T { 128 }

fn main() {}
// error: `128` may not fit in `T`: a literal of a type parameter must fit in every integer type, so be between 0 and 127
