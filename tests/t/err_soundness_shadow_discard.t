// A program's own `Discard` doesn't let a value be thrown away: only core's
// does.
trait Discard {}

struct Fd { n: i64 }

impl Discard for Fd {}

fn main() {
    let _f = Fd { n: 1 };
}
// error: `_f` is never used up, and its type, `Fd`, isn't `core::mem::Discard`; to let it be thrown away, add `#[derive(core::mem::Discard)]` to `Fd`
