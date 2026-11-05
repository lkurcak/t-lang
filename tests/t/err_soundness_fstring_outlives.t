// A format string borrows what its holes name, so it can't outlive it.
use alloc::string::String;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let line = {
        let s = String::of(&"short-lived");
        f"<{s}>"
    };
    out.line(&line)?;
    Ok(())
}
// error: `s` doesn't live long enough
