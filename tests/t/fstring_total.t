// Writing an `f".."`, or a derived `Debug`, does only what the sink does:
// into a sink that can't fail, panic or allocate, it's total, so it can
// be in a function that says no effects at all.
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

// Its variants' names are as long as `Renamed`, so that renaming them
// (tests/golden.rs) doesn't change the lengths printed.
#[derive(Debug, Discard)]
enum Tree { Leaflet(i64), Doubled(&'static str, [u8; 2]) }

fn length(t: &Tree, k: i32) -> usize with {} {
    let mut c = Counter { n: 0 };
    f"{k} {t:?}".fmt(&mut c).into_ok();
    c.n
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&length(&Tree::Leaflet(-5), 10))?; // expect: 14
    out.line(&length(&Tree::Doubled("ab", [1, 2]), -1))?; // expect: 24
    Ok(())
}
