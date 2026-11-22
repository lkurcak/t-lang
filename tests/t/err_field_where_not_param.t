// The bound is on a type parameter of the struct, or an associated type
// of one: of any other type it's so or it isn't.
struct S<T> { t: T, n: { v: u8 | v > 0 where u8: Copy } }

fn main() {}
// error: a bound here can only be on a type parameter, `Self`, or an associated type of one, not `u8`
