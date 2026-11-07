// Files written whole and read back whole: `write` makes or empties one,
// `append` adds to its end, `remove` takes it away, and each is one call
// that gives back one `Result`.
use std::io::{Error, ErrorKind};

fn main(os: std::Os) -> Result<(), Error> with alloc {
    let fs = os.fs;
    let mut out = os.stdout;
    fs.write("notes.txt", "one\n".as_bytes())?;
    fs.append("notes.txt", "two\n".as_bytes())?;
    let s = fs.read_to_string("notes.txt")?;
    out.put(&s.as_str())?; // expect: one
    // expect: two
    s.discard();

    fs.write("notes.txt", "three".as_bytes())?;
    let bytes = fs.read("notes.txt")?;
    out.line(&bytes.len())?; // expect: 5
    bytes.discard();

    fs.remove("notes.txt")?;
    match fs.read("notes.txt") {
        Ok(b) => b.discard(),
        Err(e) => out.line(&(e.kind() == ErrorKind::NotFound))?, // expect: true
    }
    Ok(())
}
