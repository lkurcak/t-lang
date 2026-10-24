// A sink that can fail says how, by its `Error`, which `?` passes up. Here
// it's linear: a `Full` must be dealt with, not dropped.
use alloc::string::String;
use core::fmt::{Display, Write};

/// What a `Bounded` says when it has no room for what's written.
struct Full { wanted: usize }

/// A string that holds at most `room` more bytes.
struct Bounded { s: String, room: usize }

impl Write for Bounded {
    type Error = Full;
    effect e = alloc;

    fn write_str(&mut self, s: &str) -> Result<(), Full> with alloc {
        let n = s.len();
        if n > self.room {
            return Result::Err(Full { wanted: n });
        }
        self.room = self.room - n;
        self.s.push_str(s);
        Result::Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), Full> with alloc {
        let n = c.len_utf8();
        if n > self.room {
            return Result::Err(Full { wanted: n });
        }
        self.room = self.room - n;
        self.s.push(c);
        Result::Ok(())
    }
}

/// Writes `a`, then `b`, stopping at the first that doesn't fit.
fn both<A: Display, B: Display, W: Write>(a: &A, b: &B, w: &mut W) -> Result<(), W::Error>
with W::e
{
    w.put(a)?;
    w.write_char('/')?;
    w.put(b)
}

fn attempt(room: usize) with alloc {
    let mut b = Bounded { s: String::new(), room };
    let r = both(&1234i64, &"abc", &mut b);
    let Bounded { s, room: _ } = b;
    match r {
        Result::Ok(()) => print_str(s.as_str()),
        Result::Err(Full { wanted }) => {
            print_str(s.as_str());
            print(wanted);
        },
    }
    s.free();
}

fn main() with alloc {
    attempt(10); // expect: 1234/abc
    attempt(6);  // expect: 1234/
                 // expect: 3
    attempt(2);  // expect: 12
                 // expect: 1
}
