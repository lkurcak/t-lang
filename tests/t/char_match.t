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

fn main() {
    print(kind('q')); // expect: 1
    print(kind('7')); // expect: 2
    print(kind(' ')); // expect: 3
    print(kind('\u{FFFD}')); // expect: 4
}
