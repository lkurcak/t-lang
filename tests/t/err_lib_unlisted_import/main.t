// A `use` names a library as any path does.
// error: this package's `t.toml` doesn't list the `alloc` library in its `libraries`
use alloc::boxed::Box;

fn main(os: std::Os) {}
