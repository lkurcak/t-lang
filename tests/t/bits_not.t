// `!` of an integer flips every bit: of an unsigned type that's its
// greatest value minus it, of a signed one -1 minus it. Of a `bool`, it's
// as before.
fn flip<T: Int>(x: T) -> T { !x }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    let z: u8 = 0;
    out.line(&(!z))?;              // expect: 255
    out.line(&(!0x0Fu16))?;        // expect: 65520
    let s: i8 = 0;
    out.line(&(!s))?;              // expect: -1
    out.line(&(!-128i8))?;         // expect: 127
    out.line(&flip(5u32))?;      // expect: 4294967290
    out.line(&flip(5i64))?;      // expect: -6
    out.line(&(!false))?;     // expect: true
    let f = |x| !x;
    out.line(&f(1u8))?;          // expect: 254
    Result::Ok(())
}
