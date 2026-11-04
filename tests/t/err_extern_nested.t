// An `extern fn` is a module's, not a function's.
fn main() {
    extern fn f() {}
// error: an `extern fn` is a module's
}
