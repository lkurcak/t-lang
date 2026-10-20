// A `where` clause bounds a type parameter, `Self`, or a projection.
fn f() where int: Discard {}

fn main() {}
// error: a `where` clause can only bound a type parameter, `Self`, or an associated type of one, not `int`
