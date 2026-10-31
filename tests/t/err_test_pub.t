// A test isn't for other code to use, so it's not `pub` or `export`.
fn main() {}

#[test]
pub fn visible() {}
// error: a test can't be `pub`
