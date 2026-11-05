// A hole has the effects of what's in it, which its function must have:
// one that says it has none can't call what may panic there.
fn half(n: i64) -> i64 with panic {
    if n < 0 {
        panic("negative");
    }
    n / 2
}

fn length(n: i64) -> usize with {} {
    let mut c = Counter { n: 0 };
    f"{half(n)}".fmt(&mut c).into_ok();
    c.n
}

use core::fmt::{Display, Write};

#[derive(Discard)]
struct Counter { n: usize }

impl Write for Counter {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        self.n = core::num::wrapping_add(self.n, s.len());
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), !> {
        self.n = core::num::wrapping_add(self.n, c.len_utf8());
        Ok(())
    }
}

fn main() {
    let _ = length(4);
}
// error: `half` may panic
