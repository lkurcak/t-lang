//! Stopping the program, and checking what must be true on the way.

use crate::cmp::Eq;
use crate::fmt::{Display, Write};
use crate::result::Result;

/// Aborts the program, saying `why`: nothing after it runs, nothing is
/// cleaned up, and no value is left behind to use up (see
/// docs/07-effects.md). Its type is `!`, as it never returns.
#[intrinsic]
export fn panic(why: &str) -> ! with panic;

/// Aborts the program as `panic` does, saying `why` as its `Display`
/// writes it. Writing it can't fail, panic or allocate: the sink it's
/// written into can't, and `fmt` does only what its sink does.
#[blame_caller]
export fn panic_with<D: Display>(why: &D) -> ! with panic {
    let mut m = Message::new();
    why.fmt(&mut m).into_ok();
    m.abort()
}

/// Panics, saying `assertion failed`, unless `ok`.
#[blame_caller]
export fn assert(ok: bool) with panic {
    if !ok { panic("assertion failed") }
}

/// Panics unless `left == right`, saying what each is.
#[blame_caller]
export fn assert_eq<T: Eq + Display>(left: &T, right: &T) with panic {
    if !(left == right) { panic_with(&Mismatch { op: "==", left, right }) }
}

/// Panics unless `left != right`, saying what they are.
#[blame_caller]
export fn assert_ne<T: Eq + Display>(left: &T, right: &T) with panic {
    if left == right { panic_with(&Mismatch { op: "!=", left, right }) }
}

/// What a failed `assert_eq` or `assert_ne` says: what it compared, and
/// the values on each side.
#[derive(Discard)]
struct Mismatch<'a, T> { op: &'a str, left: &'a T, right: &'a T }

/// `assertion `left == right` failed`, then each side on a line of its
/// own.
impl<'a, T: Display> Display for Mismatch<'a, T> {
    #[blame_caller]
    fn fmt<W: Write>(&self, w: &mut W) -> Result<(), W::Error> with W::e {
        w.write_str("assertion `left ")?;
        w.write_str(self.op)?;
        w.write_str(" right` failed\n  left: ")?;
        w.line(self.left)?;
        w.write_str(" right: ")?;
        w.put(self.right)
    }
}

/// Where a panic's message goes as it's written, before the program
/// stops: only `panic_with` makes one, and nothing gets one past it, so
/// writing one is no way to do I/O. Writing to it can't fail.
#[intrinsic]
struct Message {}

impl Message {
    /// An empty message.
    #[intrinsic]
    fn new() -> Message;

    /// Adds `s` to the message.
    #[intrinsic]
    fn push_str(&mut self, s: &str);

    /// Adds `c` to the message.
    #[intrinsic]
    fn push_char(&mut self, c: char);

    /// Aborts the program, saying the message.
    #[intrinsic]
    fn abort(self) -> ! with panic;
}

/// Text written into the message.
impl Write for Message {
    type Error = !;
    effect e = {};

    fn write_str(&mut self, s: &str) -> Result<(), !> {
        self.push_str(s);
        Result::Ok(())
    }

    fn write_char(&mut self, c: char) -> Result<(), !> {
        self.push_char(c);
        Result::Ok(())
    }
}
