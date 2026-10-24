// `fmt` and `put` are trait methods, so the trait must be in scope; the
// prelude doesn't bring in `core::fmt`.
use alloc::string::String;

fn main() with alloc {
    let mut s = String::new();
    s.put(&1i64);
    s.discard();
}
// error: `String` has no method `put` in scope; trait `core::fmt::Write` has one: import it with `use core::fmt::Write;`
