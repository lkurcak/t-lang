// Floats on a machine with no C library: arithmetic, `sqrt`, `as`, and
// writing and reading them, which are all T's or the machine's own
// instructions, so the program needs nothing the platform doesn't give.
use core::fmt::{Display, Write};

extern fn power_off(code: i32) -> !;

/// Nothing to write a panic's message to, as none can happen.
extern fn t_platform_write(_: u8) {}

extern fn t_platform_halt() -> ! {
    // SAFETY: turning the machine off is all `power_off` does.
    unsafe { power_off(3) }
}

/// Text, in a buffer.
#[derive(Discard)]
struct Text { b: [u8; 64], n: usize }

impl Write for Text {
    type Error = ();
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), ()> {
        for c in s.as_bytes() {
            if self.n < 64 {
                self.b[self.n] = *c;
                self.n = self.n + 1;
            }
        }
        Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), ()> {
        if self.n < 64 && c as u32 < 128 {
            self.b[self.n] = c as u8;
            self.n = self.n + 1;
        }
        Ok(())
    }
}

fn main() {
    let x = 2.0f64.sqrt() * 3.0;
    let mut t = Text { b: [0; 64], n: 0 };
    let _ = Display::fmt(&x, &mut t);
    let back = match "4.242640687119286".parse::<f64>() {
        Ok(y) => y,
        Err(_) => 0.0,
    };
    let written = t.n == 17 && t.b[0] == 0x34 && t.b[16] == 0x36;
    let code = if written && back == x && (x * 10.0).floor() == 42.0 { 7 } else { 1 };
    // SAFETY: turning the machine off is all `power_off` does.
    unsafe { power_off(code) }
}
// exit: 7
