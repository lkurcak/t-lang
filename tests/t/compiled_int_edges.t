// Arithmetic at the edges of each integer type, on values the checker
// can't see, so each operator is checked as it runs: what the C backend
// computes in wider types, and must still get exactly right
// (docs/33-c-backend.md). The differential tests run it compiled too.
fn add<T: Int>(a: T, b: T) -> T with panic { a + b }
fn sub<T: Int>(a: T, b: T) -> T with panic { a - b }
fn mul<T: Int>(a: T, b: T) -> T with panic { a * b }
fn div<T: Int>(a: T, b: T) -> T with panic { a / b }
fn rem<T: Int>(a: T, b: T) -> T with panic { a % b }
fn shl<T: Int>(a: T, n: u32) -> T with panic { a << n }
fn shr<T: Int>(a: T, n: u32) -> T { a >> n }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    let min: i64 = core::num::least();
    let max: i64 = core::num::greatest();
    out.line(&add(min, max))?;            // expect: -1
    out.line(&sub(min, -1))?;             // expect: -9223372036854775807
    out.line(&sub(-1, min))?;             // expect: 9223372036854775807
    out.line(&mul(min, 1))?;              // expect: -9223372036854775808
    out.line(&mul(-4611686018427387904i64, 2))?; // expect: -9223372036854775808
    out.line(&div(min, 1))?;              // expect: -9223372036854775808
    out.line(&div(-7i64, 2))?;            // expect: -3
    out.line(&rem(-7i64, 2))?;            // expect: -1
    out.line(&rem(min, 2))?;              // expect: 0
    out.line(&shl(-1i64, 63))?;           // expect: -9223372036854775808
    out.line(&shl(0i64, 1000))?;          // expect: 0
    out.line(&shr(min, 63))?;             // expect: -1
    out.line(&shr(min, 64))?;             // expect: -1
    out.line(&shr(-7i64, 1))?;            // expect: -4
    out.line(&shr(max, 62))?;             // expect: 1
    // Narrow types, which C would compute in `int`.
    out.line(&mul(255u16, 257))?;         // expect: 65535
    out.line(&mul(181u16, 362))?;         // expect: 65522
    out.line(&mul(-128i8, 1))?;           // expect: -128
    out.line(&add(-100i8, -28))?;         // expect: -128
    out.line(&shl(1u8, 7))?;              // expect: 128
    out.line(&shl(-1i8, 7))?;             // expect: -128
    out.line(&shr(-128i8, 7))?;           // expect: -1
    out.line(&shr(200u8, 8))?;            // expect: 0
    out.line(&mul(65535u32, 65537))?;     // expect: 4294967295
    out.line(&mul(4294967295u64, 4294967297))?; // expect: 18446744073709551615
    out.line(&sub(0u64, 0))?;             // expect: 0
    out.line(&div(18446744073709551615u64, 3))?; // expect: 6148914691236517205
    // `as` at the edges.
    let big: u64 = 18446744073709551615;
    out.line(&(255u64 as u8))?;           // expect: 255
    out.line(&(big as u64))?;             // expect: 18446744073709551615
    out.line(&(min as i64))?;             // expect: -9223372036854775808
    out.line(&(-128i64 as i8))?;          // expect: -128
    out.line(&(max as u64))?;             // expect: 9223372036854775807
    out.line(&(1114111u32 as char as u32))?; // expect: 1114111
    out.line(&core::num::wrap::<i64, u8>(-1i64))?; // expect: 255
    out.line(&core::num::wrap::<u64, i8>(big))?; // expect: -1
    out.line(&core::num::wrap::<i32, i16>(32768))?; // expect: -32768
    out.line(&core::num::checked_mul(min, -1).is_none())?; // expect: true
    out.line(&core::num::checked_mul(3u8, 85).unwrap())?; // expect: 255
    Result::Ok(())
}
