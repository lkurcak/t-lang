// The checker knows what `!`, `<<` and `>>` by a constant give, and
// enough of `&`, `|` and `^` to prove masking and packing fit: none of
// this can panic.
fn low(x: u32) -> u8 { (x & 0xFF) as u8 }

fn nibble(x: u8) -> u8 { (x & 0x0F) << 4 }

fn high(x: u16) -> u8 { (x >> 8) as u8 }

fn twice(x: { v: u8 | v <= 127 }) -> u8 { x << 1 }

fn top(x: { v: u8 | v == 0 }) -> { r: u8 | r == 255 } { !x }

fn neg_mask(x: i64) -> u8 { (x & 0x7F) as u8 }

fn small(x: { v: u16 | v <= 255 }) -> { r: u16 | r == x } { x & 0xFF }

fn join(a: { v: u8 | v <= 15 }, b: { v: u8 | v <= 15 }) -> u8 { (a << 4) | b }

fn mix(a: { v: u32 | v <= 1000 }, b: { v: u32 | v <= 1000 }) -> { r: u32 | r <= 2000 } { a ^ b }

fn half(x: i32, n: u32) -> { r: i32 | r <= 0 || r <= x } { x >> n }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&low(0x1234))?;         // expect: 52
    out.line(&nibble(0xAB))?;        // expect: 176
    out.line(&high(0xBEEF))?;        // expect: 190
    out.line(&twice(100))?;          // expect: 200
    out.line(&top(0))?;              // expect: 255
    out.line(&neg_mask(-1))?;        // expect: 127
    out.line(&small(200))?;          // expect: 200
    out.line(&join(0xA, 0x5))?;      // expect: 165
    out.line(&mix(1000, 3))?;        // expect: 1003
    out.line(&half(-7, 1))?;         // expect: -4
    Result::Ok(())
}
