// What a C function does is C's: that it keeps to what its import says is
// its caller's to know, so a call is unsafe.
extern fn abs(x: i32) -> i32;

fn main() {
    let _ = abs(-3);
// error: `abs` is unsafe, but it isn't in an `unsafe` block
}
