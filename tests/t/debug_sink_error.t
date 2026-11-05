// A derived `Debug` gives the first error its sink gives, and writes
// nothing after it: each write is `?`'d. The sink's error here is linear,
// so it must be handed back, not dropped.
use core::fmt::Write;

struct Full { why: u8 }

#[derive(Discard)]
struct Limited { left: u8, written: u8 }

impl Write for Limited {
    type Error = Full;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), Full> {
        if self.left == 0 {
            return Err(Full { why: 1 });
        }
        self.left = self.left - 1;
        self.written = core::num::wrapping_add(self.written, 1);
        let _ = s;
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), Full> {
        let _ = c;
        self.write_str("")
    }
}

#[derive(Debug, Discard)]
struct Point { x: i32, y: i32 }

#[derive(Debug, Discard)]
enum Wrap { One(Point) }

fn count(left: u8) -> u8 {
    let mut w = Limited { left, written: 0 };
    match core::fmt::Debug::fmt(&Wrap::One(Point { x: 1, y: 2 }), &mut w) {
        Ok(()) => 100 + w.written,
        Err(Full { why }) => why * 10 + w.written,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    // `One(`, `Point { x: `, `1`, `, y: `, `2`, ` }`, `)`: seven writes.
    out.line(&count(100))?; // expect: 107
    out.line(&count(7))?; // expect: 107
    out.line(&count(6))?; // expect: 16
    out.line(&count(2))?; // expect: 12
    out.line(&count(0))?; // expect: 10
    Ok(())
}
