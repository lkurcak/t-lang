// `x << n` is `x` times 2^n, and `x >> n` is `x` divided by 2^n, rounded
// down: for a negative `x`, towards minus infinity. `>>` is never too
// far: past the type's width it gives 0, or -1 for a negative number.
fn main() with panic {
    let one: u8 = 1;
    print(one << 7);        // expect: 128
    print(0xF0u8 >> 4);     // expect: 15
    let n: i32 = -5;
    print(n >> 1);          // expect: -3
    print(n << 2);          // expect: -20
    print(n >> 100);        // expect: -1
    print(5i32 >> 100);     // expect: 0
    let zero: u64 = 0;
    print(zero << 1000);    // expect: 0
    let big: u64 = 1;
    print(big << 63);       // expect: 9223372036854775808
    let k: u32 = 3;
    print(1i64 << k);       // expect: 8
}
