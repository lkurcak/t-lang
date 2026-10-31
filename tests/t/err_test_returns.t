// A test gives back `()` or a `Result<(), E>`, as `main` does.
fn main() {}

#[test]
fn number() -> u8 { 1 }
// error: a test gives back `()`, or a `Result<(), E>` whose error is reported, not `u8`
