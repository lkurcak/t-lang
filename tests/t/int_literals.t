// A literal's type is worked out from where it's used, as any other type
// the program doesn't write down.
fn byte(b: u8) -> u8 { b }

fn main() with panic {
    let x = 200;
    print(byte(x));             // expect: 200
    let y: i16 = 1 + 2 * 3;
    print(y);                   // expect: 7
    let z = byte(1) + 2;
    print(z);                   // expect: 3
    // A negated literal is a literal: the least `i8` is one.
    let m: i8 = -128;
    print(m);                   // expect: -128
    let n: u64 = 18446744073709551615;
    print(n);                   // expect: 18446744073709551615
}
