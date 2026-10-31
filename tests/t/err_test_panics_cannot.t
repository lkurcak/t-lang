// A test that passes only by panicking must be able to: one whose effects
// have no `panic` could never pass.
fn main() {}

fn total() -> u8 { 1 }

#[test(panics)]
fn never() { let _ = total(); }
// error: this test passes only if it panics, and it can't
