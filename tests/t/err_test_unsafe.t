// A test's obligations would be kept by no one, as `main`'s would.
fn main() {}

#[test]
fn risky() with unsafe {}
// error: a test can't say `with unsafe`
