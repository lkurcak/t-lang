// `wrap::<T, U>(x)` is `x` modulo 2^w, for a `U` that's `w` bits wide, in
// `U`'s range: the bits of `x`'s two's complement that fit. For every
// pair of integer types, so generic code can use it (see
// docs/12-bits.md and docs/29-closed-and-sealed-traits.md).
use core::num::wrap;

/// Any integer as the 64 bits of its two's complement.
fn bits<T: Int>(x: T) -> u64 { wrap(x) }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&wrap::<i8, u64>(-1))?;                    // expect: 18446744073709551615
    out.line(&wrap::<u16, u8>(300))?;                   // expect: 44
    out.line(&wrap::<u8, i8>(200))?;                    // expect: -56
    out.line(&wrap::<i64, i32>(-1))?;                   // expect: -1
    out.line(&wrap::<u64, i64>(18446744073709551615))?; // expect: -1
    out.line(&wrap::<i16, i64>(-300))?;                 // expect: -300
    out.line(&bits(-2i32))?;                            // expect: 18446744073709551614
    out.line(&bits(255u8))?;                            // expect: 255
    Result::Ok(())
}
