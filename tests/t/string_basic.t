// A `String` grows as characters and strings are pushed, and gives them
// back as a `&str`.
use alloc::string::String;

fn main() with {alloc, panic} {
    let mut s = String::new();
    print_bool(s.is_empty()); // expect: true
    s.push('a');
    s.push('ñ');
    s.push('€');
    s.push('😀');
    print(s.len());        // expect: 10
    print_str(s.as_str()); // expect: añ€😀
    s.push_str(" ok");
    print_str(s.as_str());     // expect: añ€😀 ok
    print(s.as_bytes().len()); // expect: 13
    let mut n = 0u32;
    for _c in s.as_str().chars() {
        n = n + 1;
    }
    print(n); // expect: 7
    s.free();

    let t = String::from("héllo");
    print(t.len());        // expect: 6
    print_str(t.as_str()); // expect: héllo
    t.free();

    let e = String::with_capacity(8);
    print_bool(e.capacity() >= 8); // expect: true
    print(e.len());                // expect: 0
    e.free();
}
