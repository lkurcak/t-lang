// `\x` is for bytes; a character is `\u{..}`.
fn main() {
    print_str("\x41");
}
// error: `\x` escapes are for byte literals; write a character as `\u{..}`
