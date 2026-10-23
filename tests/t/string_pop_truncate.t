// `pop` takes the last character out; `truncate` cuts only where a
// character starts, and says if it did.
use alloc::string::String;

fn show(o: Option<char>) {
    match o {
        Option::Some(c) => print(c as u32),
        Option::None => print_str("none"),
    }
}

fn main() with {alloc, panic} {
    let mut s = String::from("a€😀");
    show(s.pop());            // expect: 128512
    print_str(s.as_str());    // expect: a€
    show(s.pop());            // expect: 8364
    show(s.pop());            // expect: 97
    show(s.pop());            // expect: none
    print_bool(s.is_empty()); // expect: true

    s.push_str("añb");
    print_bool(s.truncate(2)); // expect: false
    print_str(s.as_str());     // expect: añb
    print_bool(s.truncate(9)); // expect: true
    print_str(s.as_str());     // expect: añb
    print_bool(s.truncate(3)); // expect: true
    print_str(s.as_str());     // expect: añ
    print_bool(s.truncate(0)); // expect: true
    print_bool(s.is_empty());  // expect: true

    s.push_str("xyz");
    let cap = s.capacity();
    s.clear();
    print(s.len());                  // expect: 0
    print_bool(s.capacity() == cap); // expect: true
    s.free();
}
