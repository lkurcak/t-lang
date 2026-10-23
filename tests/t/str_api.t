// What a `str` can do: all of it safe code over its bytes.
fn show(o: Option<&str>) {
    match o {
        Option::Some(s) => print_str(s),
        Option::None => print_str("none"),
    }
}

fn main() with panic {
    let s = "añb€";
    print(s.len()); // expect: 7
    print_bool(s.is_empty()); // expect: false
    print_bool("".is_empty()); // expect: true
    print_bool(s.is_char_boundary(2)); // expect: false
    print_bool(s.is_char_boundary(3)); // expect: true
    print_bool(s.is_char_boundary(7)); // expect: true
    print_bool(s.is_char_boundary(8)); // expect: false
    show(s.get(1, 3)); // expect: ñ
    show(s.get(1, 2)); // expect: none
    show(s.get(3, 9)); // expect: none
    match s.split_at(3) {
        Option::Some(p) => {
            print_str(p.head); // expect: añ
            print_str(p.tail); // expect: b€
        },
        Option::None => {},
    }
    let mut n = 0u32;
    for c in s.chars() {
        n = n + 1;
        print(c as u32);
    }
    // expect: 97
    // expect: 241
    // expect: 98
    // expect: 8364
    print(n); // expect: 4
    show(core::str::from_utf8(b"ok")); // expect: ok
    show(core::str::from_utf8(b"\xC3\x28")); // expect: none
    show(core::str::from_utf8(b"\xED\xA0\x80")); // expect: none
    show(core::str::from_utf8(b"\xC0\x80")); // expect: none
    show(core::str::from_utf8(b"\xF4\x90\x80\x80")); // expect: none
    show(core::str::from_utf8(b"\xF0\x9F\x98\x80")); // expect: 😀
}
