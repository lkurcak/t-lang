// The same for a character literal: `\xFF` is no character.
fn main() {
    let _ = '\xFF';
}
// error: `\xFF` isn't ASCII, so it's a byte, not a character; write the character as `\u{FF}`
