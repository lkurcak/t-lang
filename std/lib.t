//! The std library: what a program running on an operating system can
//! use, beside `core` and `alloc`. It's written in T, checked like any
//! other code. What the checker takes on trust is the streams of `io`,
//! intrinsic types, and the one intrinsic method of each, which reads or
//! writes a byte (see docs/19-std-io.md), and `thread`'s, which start and
//! join threads (docs/41-threads.md).
//!
//! Nothing here is ambient: a function can read or write only through a
//! stream it was given. They start out in the `Os` that `main` takes, and
//! no code can make one.
//!
//! Nothing here is in the prelude: a program says what it uses.

export mod env;
export mod fs;
export mod io;
pub mod rt;
export mod thread;

use crate::env::{Args, Env};
use crate::fs::Fs;
use crate::io::{Stderr, Stdin, Stdout};
use crate::thread::Threads;

/// What a program can do to the world outside it, and what it's told by
/// it: the operating system's streams, the program's arguments and
/// environment, its files, and starting threads. `fn main(os: Os)` is given the only one there is,
/// and passes on what each part of the program needs.
#[derive(Discard, Send, Sync)]
export struct Os {
    /// Where input comes from: the terminal, or a file or pipe instead.
    export stdin: Stdin,
    /// Where output goes.
    export stdout: Stdout,
    /// Where errors go, so they're seen even when the output isn't.
    export stderr: Stderr,
    /// The arguments it was started with.
    export args: Args,
    /// The variables of its environment.
    export env: Env,
    /// The files it can read and write.
    export fs: Fs,
    /// The right to start threads.
    export threads: Threads,
}
