// Not yet: an alias takes lifetime, type and const parameters.
type Callback<effect e> = fn() -> i64 with e;

fn main() {}
// error: a type alias can't have effect parameters
