//! `implicit_panics`, off unless asked for, warns of each operator whose
//! check wasn't proved to pass, so that it may panic: with
//! `#![warn(implicit_panics)]` and `--deny-warnings`, a program is checked
//! to have none. What the proof was missing is in the warning's notes.
#![warn(implicit_panics)]

// Effect inference makes this `panic`, silently but for the lint.
fn helper(i: usize) -> usize { i - 10 }

// Proved: nothing to say.
fn safe(i: { i: usize | i >= 10 }) -> usize { i - 10 }

// A function that says `with panic` has its operators' too.
fn ratio(a: u8, b: u8) -> u8 with panic { a / b }

// `#[allow(..)]` turns it off again.
#[allow(implicit_panics)]
fn allowed(x: u8) -> u8 { x * 2 }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    // In a closure, too.
    let f = |x: u8| x + 1;
    out.line(&f(3))?;        // expect: 4
    out.line(&helper(20))?;  // expect: 10
    out.line(&safe(20))?;    // expect: 10
    out.line(&ratio(9, 3))?; // expect: 3
    out.line(&allowed(4))?;  // expect: 8
    Ok(())
}
// warning: `-` may panic: it wasn't proved not to
// warning: `/` may panic: it wasn't proved not to
// warning: `+` may panic: it wasn't proved not to
