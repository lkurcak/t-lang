// Nor does writing its return type.
fn main() {
    let k = |x: i64| -> ! { return x; };
    k(1);
}
// error: expected `!`, found `i64`
