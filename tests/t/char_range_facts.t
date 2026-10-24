// A character is never a surrogate, and never past `10FFFF`, so `as u8`
// of what's below 256 can't panic, nor can arithmetic on its value.
fn not_surrogate(c: char) -> { v: u32 | v < 0xD800 || v > 0xDFFF } { c as u32 }

fn plus_one(c: char) -> u32 { c as u32 + 1 }

fn low(c: char) -> u8 { if (c as u32) < 256 { c as u8 } else { 0 } }

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&not_surrogate('\u{E000}'))?; // expect: 57344
    out.line(&plus_one('\u{10FFFF}'))?; // expect: 1114112
    out.line(&low('é'))?; // expect: 233
    Result::Ok(())
}
