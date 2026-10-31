//! A library's tests: it has no `main`, and what only its tests use isn't
//! dead.

/// Adds one.
export fn succ(x: u8) -> u8 { if x == 255 { 255 } else { x + 1 } }

fn only_for_tests() -> u8 { 41 }

#[test]
fn adds_one() { assert_eq(&succ(only_for_tests()), &42); }

#[test]
fn saturates() { assert_eq(&succ(255), &255); }

// test: adds_one ... ok
// test: saturates ... ok
