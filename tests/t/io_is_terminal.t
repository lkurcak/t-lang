// Whether each stream is a terminal: in a test, each is a pipe, or bytes
// in memory, so none is. A program that echoes its input only when it
// isn't typed reads as a transcript either way.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&os.stdin.is_terminal())?; // expect: false
    out.line(&out.is_terminal())?;      // expect: false
    out.line(&os.stderr.is_terminal())  // expect: false
}
