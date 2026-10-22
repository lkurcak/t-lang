// A `where` clause bounds a type parameter, `Self`, or a projection.
fn f() where i64: Discard {}

fn main() {}
// error: a `where` clause can only bound a type parameter, `Self`, or an associated type of one, not `i64`
