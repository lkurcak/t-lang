// `panic_with` panics saying what any `Display` writes, with no `alloc`.
fn main() { core::panic::panic_with(&42i64) }
// error: runtime error: panicked: 42
