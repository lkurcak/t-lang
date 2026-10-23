// Escapes: each means one thing. `\u{..}` is a character, written as
// UTF-8; `\x..` is a byte, only in byte literals.
fn main() {
    print_str("tab:\tend"); // expect: tab:	end
    print_str("quote: \" and \\ and \'"); // expect: quote: " and \ and '
    print("\u{20AC}".len()); // expect: 3
    print("€".len()); // expect: 3
    print("\0".len()); // expect: 1
    print(b"\xFF\x00".len()); // expect: 2
    print(b'\xFF'); // expect: 255
    print(b'a'); // expect: 97
    print(b'\n'); // expect: 10
    print('\u{1F600}' as u32); // expect: 128512
    print('\'' as u32); // expect: 39
    print("two
lines".len()); // expect: 9
}
