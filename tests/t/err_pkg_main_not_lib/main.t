// A program isn't a library: only a directory with a `lib.t` can be
// depended on.
// error: `app` is a program, with a `main.t`, not a library; a package to depend on starts in `lib.t`
fn main() {}
