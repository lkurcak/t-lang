// What a `str` can do: all of it safe code over its bytes.
fn show(out: &mut std::io::Stdout, o: Option<&str>) -> Result<(), std::io::Error> {
    match o {
        Option::Some(s) => out.line(&s),
        Option::None => out.line(&"none"),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let s = "añb€";
    out.line(&s.len())?; // expect: 7
    out.line(&s.is_empty())?; // expect: false
    out.line(&"".is_empty())?; // expect: true
    out.line(&s.is_char_boundary(2))?; // expect: false
    out.line(&s.is_char_boundary(3))?; // expect: true
    out.line(&s.is_char_boundary(7))?; // expect: true
    out.line(&s.is_char_boundary(8))?; // expect: false
    show(&mut out, s.get(1, 3))?; // expect: ñ
    show(&mut out, s.get(1, 2))?; // expect: none
    show(&mut out, s.get(3, 9))?; // expect: none
    match s.split_at(3) {
        Option::Some(p) => {
            out.line(&p.head)?; // expect: añ
            out.line(&p.tail)?; // expect: b€
        },
        Option::None => {},
    }
    let mut n = 0u32;
    for c in s.chars() {
        n = n + 1;
        out.line(&(c as u32))?;
    }
    // expect: 97
    // expect: 241
    // expect: 98
    // expect: 8364
    out.line(&n)?; // expect: 4
    show(&mut out, core::str::from_utf8(b"ok"))?; // expect: ok
    show(&mut out, core::str::from_utf8(b"\xC3\x28"))?; // expect: none
    show(&mut out, core::str::from_utf8(b"\xED\xA0\x80"))?; // expect: none
    show(&mut out, core::str::from_utf8(b"\xC0\x80"))?; // expect: none
    show(&mut out, core::str::from_utf8(b"\xF4\x90\x80\x80"))?; // expect: none
    show(&mut out, core::str::from_utf8(b"\xF0\x9F\x98\x80"))?; // expect: 😀
    Result::Ok(())
}
