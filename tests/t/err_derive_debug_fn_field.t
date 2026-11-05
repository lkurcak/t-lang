// A derived `Debug` writes each field by its own, and a function has none.
#[derive(Debug, Discard)]
struct Handler { f: fn(i64) -> i64 }

fn main() {}
// error: `Handler` can't be `core::fmt::Debug`: its field `f` holds a `fn(i64) -> i64`, which isn't
