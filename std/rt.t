//! What happens around `main`: how the program reports the error `main`
//! gives back.

use core::fmt::Display;

use crate::io::Stderr;

/// Writes `error: `, then `e`, then a line break, to `err`: what's shown
/// when `main` gives back `Err(e)`, before the program stops with a
/// status that says it failed. If writing fails, there's nowhere left to
/// say so.
pub fn report<E: Display>(e: &E, mut err: Stderr) {
    let _ = err.put(&"error: ");
    let _ = err.line(e);
}
