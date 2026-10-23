// A surrogate is half of a UTF-16 pair, not a character.
fn main() {
    let c = '\u{D800}';
}
// error: `\u{D800}` isn't a Unicode scalar value
