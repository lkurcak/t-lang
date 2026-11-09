// A closure's parameter must match every value: `Some(x)` doesn't.
fn main() {
    let f = |Some(x): Option<u8>| x;
    let _ = f(None);
}
// error: refutable pattern in closure parameter: `Option::None` not covered
