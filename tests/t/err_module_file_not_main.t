// Only a program that starts in `main.t` has modules in files of their own,
// so a file's program is the nearest `main.t` above it.
mod m; // error: only a program that starts in `main.t`, or a library in `lib.t`, can have
fn main() {}
