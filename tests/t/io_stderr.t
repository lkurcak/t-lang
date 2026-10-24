// What goes wrong goes to `stderr`, apart from the output, through the same
// `Write`: a function generic over the sink can write to either.
use core::fmt::{Display, Write};

fn labelled<W: Write, D: Display>(w: &mut W, label: &str, x: &D) -> Result<(), W::Error> with W::e {
    w.write_str(label)?;
    w.write_str(": ")?;
    w.line(x)
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut err = os.stderr;
    labelled(&mut out, "answer", &42i64)?;
    labelled(&mut err, "warning", &"careful")?;
    out.line(&'✓')
}
// expect: answer: 42
// stderr: warning: careful
// expect: ✓
