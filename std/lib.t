//! The std library: what a program running on an operating system can
//! use, beside `core` and `alloc`. It's written in T, checked like any
//! other code. What the checker takes on trust is its intrinsic types and
//! their intrinsic methods: the streams of `io` (docs/19-std-io.md,
//! docs/47-io.md), with the terminal's raw mode and size; `env`'s
//! arguments and variables; `fs`'s files (docs/48-files.md); `time`'s
//! clock and `entropy`'s bytes (docs/49-time-entropy-terminal.md); and
//! `thread`'s, which start and join threads (docs/41-threads.md).
//!
//! Nothing here is ambient: a function can read or write, tell the time,
//! or take entropy only through what it was given. It all starts out in
//! the `Os` that `main` takes, and no code can make any of it.
//!
//! Nothing here is in the prelude: a program says what it uses.

export mod entropy;
export mod env;
export mod fs;
export mod io;
pub mod rt;
export mod thread;
export mod time;

use crate::entropy::Entropy;
use crate::env::{Args, Env};
use crate::fs::Fs;
use crate::io::{Stderr, Stdin, Stdout};
use crate::thread::Threads;
use crate::time::Clock;

/// What a program can do to the world outside it, and what it's told by
/// it: the operating system's streams, the program's arguments and
/// environment, its files, the time, unguessable bytes, and starting
/// threads. `fn main(os: Os)` is given the only one there is, and passes
/// on what each part of the program needs.
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
    /// What time it is.
    export clock: Clock,
    /// Bytes no one can guess.
    export entropy: Entropy,
    /// The right to start threads.
    export threads: Threads,
}
