// A package with a `t.toml` uses only the libraries built in that it
// lists: with no `libraries`, that's core alone, so `std` isn't there.
// error: this package's `t.toml` doesn't list the `std` library in its `libraries`
fn main(os: std::Os) {}
