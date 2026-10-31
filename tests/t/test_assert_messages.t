// A failed `assert_eq` or `assert_ne` says both sides, by their `Display`,
// and it's shown where it was called, not in the core library: the same
// for a panic in any function of core, alloc or std, such as `unwrap`.
fn main() {}

#[test]
fn eq() {
    let s = "abc";
    assert_eq(&s, &"abd");
}

#[test]
fn ne() { assert_ne(&'x', &'x'); }

#[test]
fn unwraps() {
    let x: Option<u8> = Option::None;
    let _ = x.unwrap();
}

#[test]
fn deep() { helper(); }

fn helper() { assert_eq(&1u8, &2); }

// test: eq ... FAILED: runtime error: panicked: assertion `left == right` failed left: abc right: abd
// test: ne ... FAILED: assertion `left != right` failed left: x right: x
// test: unwraps ... FAILED: runtime error: panicked: `unwrap` of `None`
// test: deep ... FAILED: left: 1 right: 2
