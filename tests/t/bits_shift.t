// `x << n` is `x` times 2^n, and `x >> n` is `x` divided by 2^n, rounded
// down: for a negative `x`, towards minus infinity. `>>` is never too
// far: past the type's width it gives 0, or -1 for a negative number.
fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let one: u8 = 1;
    out.line(&(one << 7))?;        // expect: 128
    out.line(&(0xF0u8 >> 4))?;     // expect: 15
    let n: i32 = -5;
    out.line(&(n >> 1))?;          // expect: -3
    out.line(&(n << 2))?;          // expect: -20
    out.line(&(n >> 100))?;        // expect: -1
    out.line(&(5i32 >> 100))?;     // expect: 0
    let zero: u64 = 0;
    out.line(&(zero << 1000))?;    // expect: 0
    let big: u64 = 1;
    out.line(&(big << 63))?;       // expect: 9223372036854775808
    let k: u32 = 3;
    out.line(&(1i64 << k))?;       // expect: 8
    Result::Ok(())
}
