// `\u{..}` is a character: in a byte literal, write the byte.
fn main() {
    print(b"\u{41}".len());
}
// error: `\u{..}` escapes are for characters; write a byte as `\x..`
