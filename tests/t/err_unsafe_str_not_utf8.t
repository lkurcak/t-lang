// A broken promise is caught as the program runs.
fn main() {
    let b = b"a\xC3";
    // SAFETY: wrong: `\xC3` starts a character that never ends.
    let s = unsafe { core::str::from_utf8_unchecked(b) };
    print(s.len());
}
// error: unsafe code broke its obligation: `str::from_utf8_unchecked` of bytes that aren't UTF-8
