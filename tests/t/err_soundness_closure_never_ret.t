// Nor does writing its return type.
fn main() {
    let k = |x: int| -> ! { return x; };
    k(1);
}
// error: expected `!`, found `int`
