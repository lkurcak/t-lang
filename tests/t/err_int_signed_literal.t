fn f<T: Signed>() -> T { -129 }

fn main() {}
// error: `-129` may not fit in `T`: a literal of a type parameter that's `Signed` must fit in every signed integer type, so be between -128 and 127
