// A test is live, as `main` is, and so is what only tests use; what
// nothing uses is still dead.
fn main() {}

fn helper() -> u8 { 1 }

fn unused() -> u8 { 2 } // warning: function `unused` is never used

#[test]
fn uses_helper() { assert_eq(&helper(), &1); }

// test: uses_helper ... ok
