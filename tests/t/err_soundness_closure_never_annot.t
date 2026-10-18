// Saying the closure is a `fn() -> !` doesn't make it one.
fn main() {
    let k: fn() -> ! = || return 1;
    k();
}
// error: expected `!`, found `int`
