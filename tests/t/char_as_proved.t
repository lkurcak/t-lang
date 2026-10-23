// What's proved a scalar value converts without a panic.
fn f(n: { v: u32 | v < 0xD800 || v >= 0xE000 && v <= 0x10FFFF }) -> char { n as char }

fn byte(b: u8) -> char { b as char }

fn main() {
    print(f(0x20AC) as u32); // expect: 8364
    print(byte(255) as u32); // expect: 255
}
