fn main() with panic {
    let mut x: u8 = 0b1111_0000;
    x &= 0b1010_1010;
    print(x);   // expect: 160
    x |= 1;
    print(x);   // expect: 161
    x ^= 0xFF;
    print(x);   // expect: 94
    x >>= 2;
    print(x);   // expect: 23
    x <<= 3;
    print(x);   // expect: 184
}
