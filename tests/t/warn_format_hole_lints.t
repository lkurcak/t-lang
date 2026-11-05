//! The two lints of holes are off unless asked for: here, for all the
//! program, by `#![warn(..)]` at the top of the crate root.
//! `format_hole_expressions` warns of each hole that holds more than a
//! name or a field of one; `format_hole_effects` of each thing in a hole
//! that may do more than give its value, as its function's effects say,
//! inferred ones included, once they're known.
#![warn(format_hole_expressions, format_hole_effects)]

#[derive(Discard)]
struct P { x: i64 }

fn half(n: i64) -> i64 {
    if n < 0 {
        panic("negative");
    }
    n / 2
}

fn quiet(n: i64) -> i64 { n }

fn shown(p: &P, n: i64, out: &mut std::io::Stdout) -> Result<(), std::io::Error> with panic {
    out.line(&f"{p.x} {n} {half(n)} {quiet(n)}")?;
    // `n / 2` can't panic, so it's only an expression; `n + 1` may.
    out.line(&f"{n / 2} {n + 1}")
}

// `#[allow(..)]` turns them off again inside.
#[allow(format_hole_expressions, format_hole_effects)]
fn quietly(n: i64, out: &mut std::io::Stdout) -> Result<(), std::io::Error> with panic {
    out.line(&f"{half(n)}")
}

mod inner {
    #![allow(format_hole_effects)]

    pub fn shown(n: i64, out: &mut std::io::Stdout) -> Result<(), std::io::Error> with panic {
        out.line(&f"{crate::half(n)}")
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    shown(&P { x: 1 }, 4, &mut out)?; // expect: 1 4 2 4
    // expect: 2 5
    quietly(4, &mut out)?; // expect: 2
    inner::shown(4, &mut out)?; // expect: 2
    std::Ok(())
}
// warning: this hole holds more than a name or a field of one
// warning: `half` may panic in a hole of a format string
// warning: this hole holds more than a name or a field of one
// warning: this hole holds more than a name or a field of one
// warning: this hole holds more than a name or a field of one
// warning: `+` may panic in a hole of a format string
// warning: this hole holds more than a name or a field of one
