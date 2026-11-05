// Trimming whitespace, and looking for a character or a string: at the
// start, at the end, or anywhere.
fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&"  hi there \n".trim())?;                     // expect: hi there
    out.line(&"[".len())?;                                  // expect: 1
    out.line(&" \t x ".trim_start().len())?;                // expect: 2
    out.line(&" x \t ".trim_end().len())?;                  // expect: 2
    out.line(&"\u{a0}\u{2003}x\u{3000}".trim())?;           // expect: x
    out.line(&"   ".trim().is_empty())?;                    // expect: true
    out.line(&"hello".starts_with("he"))?;                  // expect: true
    out.line(&"hello".starts_with('h'))?;                   // expect: true
    out.line(&"hello".starts_with("lo"))?;                  // expect: false
    out.line(&"hello".ends_with("lo"))?;                    // expect: true
    out.line(&"héllo→".ends_with('→'))?;                    // expect: true
    out.line(&"hello".ends_with('l'))?;                     // expect: false
    out.line(&"x".starts_with(""))?;                        // expect: true
    out.line(&"--name".strip_prefix("--").unwrap_or("?"))?; // expect: name
    out.line(&"name".strip_prefix("--").is_none())?;        // expect: true
    out.line(&"file.t".strip_suffix(".t").unwrap_or("?"))?; // expect: file
    out.line(&"→x".strip_prefix('→').unwrap_or("?"))?;      // expect: x
    out.line(&"x→".strip_suffix('→').unwrap_or("?"))?;      // expect: x
    out.line(&"a-b-c".find('-').unwrap_or(99))?;            // expect: 1
    out.line(&"aé-b".find('-').unwrap_or(99))?;             // expect: 3
    out.line(&"abcabd".find("abd").unwrap_or(99))?;         // expect: 3
    out.line(&"abc".find("abcd").unwrap_or(99))?;           // expect: 99
    out.line(&"abc".find("").unwrap_or(99))?;               // expect: 0
    out.line(&"abc".contains("bc"))?;                       // expect: true
    out.line(&"abc".contains('z'))?;                        // expect: false
    Result::Ok(())
}
