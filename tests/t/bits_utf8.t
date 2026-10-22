// Encoding a code point as UTF-8: masks, shifts and `as`, all proved not
// to panic.
fn width(c: { v: u32 | v <= 0x10FFFF }) -> u8 {
    if c < 0x80 { 1 } else if c < 0x800 { 2 } else if c < 0x10000 { 3 } else { 4 }
}

fn lead(c: { v: u32 | v <= 0x10FFFF }) -> u8 {
    if c < 0x80 {
        c as u8
    } else if c < 0x800 {
        (c >> 6) as u8 | 0xC0
    } else if c < 0x10000 {
        (c >> 12) as u8 | 0xE0
    } else {
        (c >> 18) as u8 | 0xF0
    }
}

fn cont(c: u32, shift: u32) -> u8 { ((c >> shift) & 0x3F) as u8 | 0x80 }

fn main() {
    print(width(0x41));     // expect: 1
    print(lead(0x41));      // expect: 65
    print(width(0xE9));     // expect: 2
    print(lead(0xE9));      // expect: 195
    print(cont(0xE9, 0));   // expect: 169
    print(lead(0x20AC));    // expect: 226
    print(cont(0x20AC, 6)); // expect: 130
    print(cont(0x20AC, 0)); // expect: 172
    print(lead(0x1F600));   // expect: 240
}
