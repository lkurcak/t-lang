// All of a stream at once: the input's bytes, into a list, or its text,
// into a string, which fails if it isn't UTF-8 and leaves the string be.
use alloc::io::{read_to_end, read_to_string};
use alloc::list::List;
use alloc::string::String;
use core::io::{Bytes, ErrorKind};

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut input = os.stdin;
    let mut out = os.stdout;
    let mut all = String::from("[");
    let n = read_to_string(&mut input, &mut all)?;
    out.line(&n)?;           // expect: 9
    out.put(&all.as_str())?; // expect: [one
    out.line(&"]")?;         // expect: twö
    // expect: ]
    all.discard();

    let bad = [0x61u8, 0xFF, 0x62];
    let mut s = String::from("kept");
    match read_to_string(&mut Bytes::new(bad.as_slice()), &mut s) {
        Ok(_) => out.line(&"read")?,
        Err(e) => out.line(&(e.kind() == ErrorKind::InvalidData))?, // expect: true
    }
    out.line(&s.as_str())?; // expect: kept
    s.discard();

    let mut bytes = List::new();
    let k = read_to_end(&mut Bytes::new(bad.as_slice()), &mut bytes)?;
    out.line(&k)?;         // expect: 3
    out.line(&bytes.len()) // expect: 3
}
// stdin: one
// stdin: twö
