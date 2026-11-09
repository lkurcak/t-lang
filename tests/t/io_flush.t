// Flushing writes out what waits in a buffer: of the output, and where
// errors go, which has none, so both just work, with or without `Write`.
use core::io::Write;

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let mut err = os.stderr;
    out.put("prompt> ")?;
    out.flush()?;
    Write::flush(&mut out)?;
    err.line("note")?; // stderr: note
    err.flush()?;
    out.line("done") // expect: prompt> done
}
