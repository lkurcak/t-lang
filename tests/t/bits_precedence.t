// A chain of one operator needs no parentheses, nor does a comparison of
// their results; mixing two of `&`, `|`, `^`, the shifts and arithmetic
// does.
fn main() {
    let a: u8 = 1;
    let b: u8 = 2;
    let c: u8 = 4;
    print(a | b | c);               // expect: 7
    print_bool(a & b == 0);         // expect: true
    print((a | b) & c);             // expect: 0
    print(a | (b & c));             // expect: 1
    print((a + b) << 2);            // expect: 12
    let x: u16 = 0x1234;
    print((x >> 8) as u8 & 0x0F);   // expect: 2
}
