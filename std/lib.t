//! The std library: what a program running on an operating system can
//! use, beside `core` and `alloc`. It's written in T, checked like any
//! other code. What the checker takes on trust is the streams of `io`,
//! intrinsic types, and the one intrinsic method of each, which reads or
//! writes a byte (see docs/19-std-io.md).
//!
//! Nothing here is ambient: a function can read or write only through a
//! stream it was given. They start out in the `Os` that `main` takes, and
//! no code can make one.
//!
//! Nothing here is in the prelude: a program says what it uses.

export mod io;
export mod rt;

use crate::io::{Stderr, Stdin, Stdout};

/// What a program can do to the world outside it: the operating system's
/// streams, for now. `fn main(os: Os)` is given the only one there is,
/// and passes on what each part of the program needs.
#[derive(Discard)]
export struct Os {
    /// Where input comes from: the terminal, or a file or pipe instead.
    export stdin: Stdin,
    /// Where output goes.
    export stdout: Stdout,
    /// Where errors go, so they're seen even when the output isn't.
    export stderr: Stderr,
}
