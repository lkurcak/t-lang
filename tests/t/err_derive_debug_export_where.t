// An exported struct whose fields aren't says what its derived `Debug`
// needs, as for any derived trait: its fields are no part of its
// package's interface.
#[derive(Debug)]
export struct Wrap<T> { inner: Option<T> }

fn main() {}
// error: `Wrap` is exported with fields that aren't, so its `core::fmt::Debug` must say what it needs: `where T: core::fmt::Debug`
