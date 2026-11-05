// A digit's value in a radix, and what's whitespace: Unicode's
// White_Space, which `trim` and `split_whitespace` go by.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&'7'.to_digit(10).unwrap_or(99))?; // expect: 7
    out.line(&'f'.to_digit(16).unwrap_or(99))?; // expect: 15
    out.line(&'F'.to_digit(16).unwrap_or(99))?; // expect: 15
    out.line(&'z'.to_digit(36).unwrap_or(99))?; // expect: 35
    out.line(&'8'.to_digit(8).unwrap_or(99))?;  // expect: 99
    out.line(&'g'.to_digit(16).unwrap_or(99))?; // expect: 99
    out.line(&'1'.to_digit(2).unwrap_or(99))?;  // expect: 1
    out.line(&' '.is_whitespace())?;            // expect: true
    out.line(&'\u{85}'.is_whitespace())?;       // expect: true
    out.line(&'\u{2028}'.is_whitespace())?;     // expect: true
    out.line(&'\u{200b}'.is_whitespace())?;     // expect: false
    out.line(&'x'.is_whitespace())?;            // expect: false
    Ok(())
}
