// A panic in a `#[blame_caller]` function, as core's `assert_eq` and
// `unwrap` are, is shown where it was called: a test's helper says it too
// to be blamed in the test.
fn main() {}

#[test]
fn eq() { assert_eq(&1u8, &2); }

#[test]
fn unwraps() {
    let x: Option<u8> = Option::None;
    let _ = x.unwrap();
}

#[test]
fn through_a_helper() { helper(); }

fn helper() { assert(false); }

#[test]
fn through_a_blaming_helper() { blaming_helper(); }

#[blame_caller]
fn blaming_helper() { assert(false); }

// Through a closure the core library calls: the closure's own line.
#[test]
fn in_a_closure() {
    let x: Option<u8> = Option::Some(1);
    let _ = x.map(|v| { assert(v == 2); v });
}

// test: eq ... FAILED: test_blame.t:7:11
// test: unwraps ... FAILED: test_blame.t:12:13
// test: through_a_helper ... FAILED: test_blame.t:18:15
// test: through_a_blaming_helper ... FAILED: test_blame.t:21:33
// test: in_a_closure ... FAILED: test_blame.t:30:25
