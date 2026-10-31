// Nothing could say what a test's type parameters are.
fn main() {}

#[test]
fn generic<T>() {}
// error: a test has no generic parameters
