// A list of bytes is a `Write`: what's written goes on its end. Text goes
// in with `put` and `line`, as into any sink, in UTF-8.
use alloc::list::List;
use alloc::string::String;
use core::io::Write;

fn greet<W: Write>(w: &mut W, name: &str) -> Result<(), std::io::Error> with W::e {
    w.put("héllo, ")?;
    w.put(&name)?;
    w.line(&'!')?;
    w.write_all("bye".as_bytes())?;
    w.flush()
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut bytes = List::new();
    greet(&mut bytes, "ada")?;
    out.line(&bytes.len())?; // expect: 16
    match String::from_utf8(bytes) {
        Ok(s) => {
            out.put(&s.as_str())?; // expect: héllo, ada!
            s.discard();
        },
        Err(_) => {},
    }
    out.line("")?; // expect: bye
    // Stdout is a byte sink too: `Write::put` through `&mut`.
    greet(&mut &mut out, "stdout")?; // expect: héllo, stdout!
    out.line("")                    // expect: bye
}
