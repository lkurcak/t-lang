// Effects it says count, as inferred ones do: `alloc` isn't `panic`, and
// running out of memory doesn't pass a test that should panic.
fn main() {}

#[test(panics)]
fn never() with alloc { let _ = alloc::boxed::Box::new(1u8); }
// error: this test passes only if it panics, and it can't
