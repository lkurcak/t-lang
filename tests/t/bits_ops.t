// `&`, `|` and `^` work bit by bit, on two's complement: a negative
// number has infinitely many 1s on its left. Their result always fits in
// the type, so they never panic.
fn main() {
    let a: u8 = 0b1100;
    let b: u8 = 0b1010;
    print(a & b);           // expect: 8
    print(a | b);           // expect: 14
    print(a ^ b);           // expect: 6
    let m: i8 = -1;
    let n: i8 = 5;
    print(m & n);           // expect: 5
    print(m | n);           // expect: -1
    print(m ^ n);           // expect: -6
    let lo: i8 = -128;
    print(lo & -1);         // expect: -128
    print(lo | 127);        // expect: -1
    let w: u64 = 0xFFFF_FFFF_FFFF_FFFF;
    print(w ^ 0xFF);        // expect: 18446744073709551360
    print(0x0Fu32 & 0x3C);  // expect: 12
}
