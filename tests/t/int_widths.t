// The ten integer types, each the integers between its least and its
// greatest. `+` and the others take two of the same type and give that
// type.
fn main() {
    let a: i8 = -128;
    let b: u8 = 255;
    let c: i16 = -32768;
    let d: u16 = 65535;
    let e: i32 = -2147483647 - 1;
    let f: u32 = 4294967295;
    let g: i64 = -9223372036854775807 - 1;
    let h: u64 = 18446744073709551615;
    let i: isize = -5;
    let j: usize = 5;
    print(a);       // expect: -128
    print(b);       // expect: 255
    print(c);       // expect: -32768
    print(d);       // expect: 65535
    print(e);       // expect: -2147483648
    print(f);       // expect: 4294967295
    print(g);       // expect: -9223372036854775808
    print(h);       // expect: 18446744073709551615
    print(i * 2);   // expect: -10
    print(j + 1);   // expect: 6
    print(b - 5);   // expect: 250
    print(h / 3);   // expect: 6148914691236517205
}
