// `bool`, `char`, `str` and `String` are `Display`, and so is a reference
// or a box of anything that is. `String::of` makes a string of one.
use alloc::boxed::Box;
use alloc::string::String;
use core::fmt::{Display, Write};

fn main() with alloc {
    let mut s = String::new();
    s.put(&true);
    s.write_char(' ');
    s.put(&false);
    s.write_char(' ');
    s.put(&'é');
    s.write_char(' ');
    s.put(&"str");
    s.write_str(" ");
    let t = String::from("String");
    s.put(&t);
    t.free();
    s.write_char(' ');
    s.put(&&&42i64);
    s.write_char(' ');
    let mut n = 7u8;
    s.put(&&mut n);
    s.write_char(' ');
    s.put(&Box::new(-1i32));
    print_str(s.as_str()); // expect: true false é str String 42 7 -1
    s.free();

    let o = String::of(&123456u32);
    print_str(o.as_str()); // expect: 123456
    print(o.len());        // expect: 6
    o.free();

    // `fmt` itself, with the sink as an argument.
    let mut u = String::new();
    'x'.fmt(&mut u);
    "yz".fmt(&mut u);
    print_str(u.as_str()); // expect: xyz
    u.free();
}
