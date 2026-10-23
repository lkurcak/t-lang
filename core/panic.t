// Stopping the program.

/// Aborts the program, saying `why`: nothing after it runs, nothing is
/// cleaned up, and no value is left behind to use up (see
/// docs/07-effects.md). Its type is `!`, as it never returns.
#[intrinsic]
pub fn panic(why: &str) -> ! with panic;
