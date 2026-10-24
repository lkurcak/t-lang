// Character patterns: literals and ranges, exhaustive over the scalar
// values, which have a gap where the surrogates are.
fn kind(c: char) -> u8 {
    match c {
        'a'..='z' | 'A'..='Z' => 1,
        '0'..='9' => 2,
        '\0'..='/' | ':'..='@' | '['..='`' | '{'..='\u{D7FF}' => 3,
        '\u{E000}'..='\u{10FFFF}' => 4,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> {
    let mut out = os.stdout;
    out.line(&kind('q'))?; // expect: 1
    out.line(&kind('7'))?; // expect: 2
    out.line(&kind(' '))?; // expect: 3
    out.line(&kind('\u{FFFD}'))?; // expect: 4
    Result::Ok(())
}
