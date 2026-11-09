// What a `char` can do.
use alloc::list::List;

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    match char::from_u32(0x41) {
        Some(c) => out.line(&(c as u32))?, // expect: 65
        None => {},
    }
    match char::from_u32(0xDFFF) {
        Some(_) => out.line(&1u8)?,
        None => out.line(&0u8)?, // expect: 0
    }
    out.line(&'a'.len_utf8())?; // expect: 1
    out.line(&'ñ'.len_utf8())?; // expect: 2
    out.line(&'€'.len_utf8())?; // expect: 3
    out.line(&'😀'.len_utf8())?; // expect: 4
    out.line(&'Q'.is_ascii_uppercase())?; // expect: true
    out.line(&'q'.is_ascii_alphabetic())?; // expect: true
    out.line(&'5'.is_ascii_digit())?; // expect: true
    out.line(&'é'.is_ascii())?; // expect: false
    let mut buf: List<u8> = List::new();
    buf.push(0);
    buf.push(0);
    buf.push(0);
    let w = encode(&mut out, &mut buf);
    buf.discard();
    w
}

fn encode(out: &mut std::io::Stdout, buf: &mut List<u8>) -> Result<(), std::io::Error> {
    match '€'.encode_utf8(buf.as_mut_slice()) {
        Some(s) => out.line(&s)?, // expect: €
        None => {},
    }
    match '😀'.encode_utf8(buf.as_mut_slice()) {
        Some(s) => out.line(&s),
        None => out.line("too short"), // expect: too short
    }
}
