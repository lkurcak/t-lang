// What a `char` can do.
use alloc::list::List;

fn main() with alloc {
    match char::from_u32(0x41) {
        Option::Some(c) => print(c as u32), // expect: 65
        Option::None => {},
    }
    match char::from_u32(0xDFFF) {
        Option::Some(_) => print(1u8),
        Option::None => print(0u8), // expect: 0
    }
    print('a'.len_utf8()); // expect: 1
    print('ñ'.len_utf8()); // expect: 2
    print('€'.len_utf8()); // expect: 3
    print('😀'.len_utf8()); // expect: 4
    print_bool('Q'.is_ascii_uppercase()); // expect: true
    print_bool('q'.is_ascii_alphabetic()); // expect: true
    print_bool('5'.is_ascii_digit()); // expect: true
    print_bool('é'.is_ascii()); // expect: false
    let mut buf: List<u8> = List::new();
    buf.push(0);
    buf.push(0);
    buf.push(0);
    match '€'.encode_utf8(buf.as_mut_slice()) {
        Option::Some(s) => print_str(s), // expect: €
        Option::None => {},
    }
    match '😀'.encode_utf8(buf.as_mut_slice()) {
        Option::Some(s) => print_str(s),
        Option::None => print_str("too short"), // expect: too short
    }
    buf.free();
}
