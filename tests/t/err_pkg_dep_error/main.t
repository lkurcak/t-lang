// A dependency that doesn't check is an error, in its own files.
fn main() {
    let _a = broken::f();
}
