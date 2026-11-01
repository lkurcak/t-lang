// `1i8 << 7` is 128, which isn't an `i8`: bits shifted out are an
// overflow, though C would keep the low bits.
fn shl(a: i8, n: u32) -> i8 with panic { a << n }

fn main(os: std::Os) -> Result<(), std::io::Error> with panic {
    let mut out = os.stdout;
    out.line(&shl(1, 6))?; // expect: 64
    out.line(&shl(1, 7))?;
    Result::Ok(())
}
// error: runtime error: integer overflow
