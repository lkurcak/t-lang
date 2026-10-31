// `t test` runs the program's own tests, not its dependencies'.
fn main() {}

#[test]
fn uses_dep() { assert_eq(&dep::one(), &1); }

// test: uses_dep ... ok
