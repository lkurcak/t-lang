// What a library has `pub` is its own: `std::rt` is how the program's
// runtime reports `main`'s error, not something a program can call.
fn main(os: std::Os) {
    std::rt::report(&1i64, os.stderr);
}
// error: module `rt` isn't exported by `std`
