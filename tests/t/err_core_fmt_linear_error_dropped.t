// A sink's error that isn't `Discard` can't be dropped: here the result of
// `put` is thrown away, and with it, maybe, a `Lost`.
use core::fmt::Write;

struct Lost {}

#[derive(Discard)]
struct Never {}

impl Write for Never {
    type Error = Lost;
    effect e = {};

    fn write_str(&mut self, _s: &str) -> Result<(), Lost> { Result::Err(Lost {}) }

    fn write_char(&mut self, _c: char) -> Result<(), Lost> { Result::Err(Lost {}) }
}

fn main() {
    let mut n = Never {};
    n.put(&1u8);
}
// error: this value is never used up, and its type, `Result<(), Lost>`, isn't `Discard`; to let it be thrown away, add `#[derive(Discard)]` to `Lost`
