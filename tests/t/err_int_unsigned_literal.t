// Of a `T: Unsigned`, a literal must fit in every unsigned type: 0 to 255.
fn f<T: Unsigned>() -> T { 256 }

fn main() {}
// error: `256` may not fit in `T`: a literal of a type parameter that's `Unsigned` must fit in every unsigned integer type, so be between 0 and 255
