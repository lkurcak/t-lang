// Making a `str` of bytes nobody checked is unsafe: they might not be
// UTF-8.
fn main() {
    let b = b"\xFF";
    let s = core::str::from_utf8_unchecked(b);
    print_str(s);
}
// error: `core::str::from_utf8_unchecked` is unsafe, but it isn't in an `unsafe` block
