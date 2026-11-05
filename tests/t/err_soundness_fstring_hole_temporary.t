// What a hole works out is a temporary, dropped at the end of the
// statement: a format string kept past it would point at nothing.
use alloc::string::String;

fn make() -> String with alloc { String::of(&"temporary") }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let line = f"<{make()}>";
    out.line(&line)?;
    Ok(())
}
// error: a temporary value is dropped here while still borrowed
