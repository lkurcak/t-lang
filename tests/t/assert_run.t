// The assertions are in the prelude, for any code, not only tests.
fn main() {
    assert(true);
    assert_eq(&1i64, &1);
    assert_ne(&true, &false);
    assert_eq(&2u8, &3);
}
// error: runtime error: panicked: assertion `left == right` failed
