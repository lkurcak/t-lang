// `{x:x}` writes an integer in hexadecimal, and `{x:08x}` with at least 8
// digits, 0s in front; a negative one as its two's complement bits, as
// many as its type has.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let (a, b, c) = (255u8, -1i8, 0xdead_beefu32);
    out.line(&f"{a:x} {b:x} {c:x}")?; // expect: ff ff deadbeef
    let (z, n, m) = (0u64, -2i64, 7usize);
    out.line(&f"{z:x} {z:04x} {n:x}")?; // expect: 0 0000 fffffffffffffffe
    out.line(&f"{m:08x}|{a:01x}|{c:04x}")?; // expect: 00000007|ff|deadbeef
    let i = core::num::least::<i16>();
    out.line(&f"{i:x} {i + 1:x}") // expect: 8000 8001
}
