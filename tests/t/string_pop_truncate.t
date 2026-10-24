// `pop` takes the last character out; `truncate` cuts only where a
// character starts, and says if it did.
use alloc::string::String;
use core::fmt::Write;

fn show(out: &mut String, o: Option<char>) with alloc {
    match o {
        Option::Some(c) => out.line(&(c as u32)).into_ok(),
        Option::None => out.line(&"none").into_ok(),
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = String::new();
    let mut s = String::from("a€😀");
    show(&mut out, s.pop());            // expect: 128512
    out.line(&s.as_str());    // expect: a€
    show(&mut out, s.pop());            // expect: 8364
    show(&mut out, s.pop());            // expect: 97
    show(&mut out, s.pop());            // expect: none
    out.line(&s.is_empty()); // expect: true

    s.push_str("añb");
    out.line(&s.truncate(2)); // expect: false
    out.line(&s.as_str());     // expect: añb
    out.line(&s.truncate(9)); // expect: true
    out.line(&s.as_str());     // expect: añb
    out.line(&s.truncate(3)); // expect: true
    out.line(&s.as_str());     // expect: añ
    out.line(&s.truncate(0)); // expect: true
    out.line(&s.is_empty());  // expect: true

    s.push_str("xyz");
    let cap = s.capacity();
    s.clear();
    out.line(&s.len());                  // expect: 0
    out.line(&(s.capacity() == cap)); // expect: true
    s.free();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.free();
    w
}
