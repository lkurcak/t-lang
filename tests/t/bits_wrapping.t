// Arithmetic modulo 2^width, for hashes and checksums: written in T over
// `Int`, and proved not to panic, but for `wrapping_mul`, which is built
// in.
use core::num::{wrapping_add, wrapping_mul, wrapping_neg, wrapping_sub};

fn fnv(a: u8, b: u8) -> u32 {
    let mut h: u32 = 2166136261;
    h = wrapping_mul(h ^ a as u32, 16777619);
    h = wrapping_mul(h ^ b as u32, 16777619);
    h
}

fn main() {
    print(wrapping_add(250u8, 10));     // expect: 4
    print(wrapping_add(100u8, 10));     // expect: 110
    print(wrapping_add(127i8, 1));      // expect: -128
    print(wrapping_add(-128i8, -1));    // expect: 127
    print(wrapping_sub(3u8, 5));        // expect: 254
    print(wrapping_sub(-128i8, 1));     // expect: 127
    print(wrapping_sub(127i8, -1));     // expect: -128
    print(wrapping_neg(1u8));           // expect: 255
    print(wrapping_neg(0u8));           // expect: 0
    print(wrapping_neg(-128i8));        // expect: -128
    print(wrapping_neg(5i8));           // expect: -5
    print(wrapping_mul(16u8, 17));      // expect: 16
    print(wrapping_mul(-128i8, -1));    // expect: -128
    print(wrapping_mul(100i8, 3));      // expect: 44
    print(fnv(1, 2));                   // expect: 3983810698
}
