// `t fmt` keeps literals as they're written: escapes, and the case of
// their digits.
fn main() {
    print('\u{10FFFF}' as u32); // expect: 1114111
    print_str("\u{41}\t\"");    // expect: A	"
    print(b"\xFF\x0a".len());   // expect: 2
    print(b'\x7F');             // expect: 127
    match 'é' {
        '\u{E9}' => print(1u8), // expect: 1
        _ => print(0u8),
    }
}
