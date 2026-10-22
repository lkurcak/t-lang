// `+` on a type parameter needs it to be an integer type.
fn f<T: Copy>(a: T) -> T with panic { a + a }

fn main() {}
// error: `+` works on integers, not on `T`; add a bound: `T: Int`
