// A panic in the core library is shown where the program called into it:
// the innermost call outside of core, alloc and std.
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

// Through a closure the core library calls: the closure's own line.
#[test]
fn in_a_closure() {
    let x: Option<u8> = Option::Some(1);
    let _ = x.map(|v| { assert(v == 2); v });
}

// test: eq ... FAILED: test_blame.t:6:11
// test: unwraps ... FAILED: test_blame.t:11:13
// test: through_a_helper ... FAILED: test_blame.t:17:15
// test: in_a_closure ... FAILED: test_blame.t:23:25
