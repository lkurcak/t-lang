//! Its tests are checked with it, but `t test` of a program that depends on
//! it runs only the program's own.

/// Gives 1.
export fn one() -> u8 { 1 }

#[test]
fn fails_but_isnt_run() { assert_eq(&one(), &2); }
