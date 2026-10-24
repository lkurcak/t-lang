// A `String` grows as characters and strings are pushed, and gives them
// back as a `&str`.
use alloc::string::String;
use core::fmt::Write;

fn main(os: std::Os) -> Result<(), std::io::Error> with {alloc, panic} {
    let mut out = String::new();
    let mut s = String::new();
    out.line(&s.is_empty()); // expect: true
    s.push('a');
    s.push('ñ');
    s.push('€');
    s.push('😀');
    out.line(&s.len());        // expect: 10
    out.line(&s.as_str()); // expect: añ€😀
    s.push_str(" ok");
    out.line(&s.as_str());     // expect: añ€😀 ok
    out.line(&s.as_bytes().len()); // expect: 13
    let mut n = 0u32;
    for _c in s.as_str().chars() {
        n = n + 1;
    }
    out.line(&n); // expect: 7
    s.discard();

    let t = String::from("héllo");
    out.line(&t.len());        // expect: 6
    out.line(&t.as_str()); // expect: héllo
    t.discard();

    let e = String::with_capacity(8);
    out.line(&(e.capacity() >= 8)); // expect: true
    out.line(&e.len());                // expect: 0
    e.discard();
    let mut stdout = os.stdout;
    let w = stdout.put(&out.as_str());
    out.discard();
    w
}
