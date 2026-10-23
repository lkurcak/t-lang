// A string's bytes can be read, never changed: `as_bytes` lends them
// shared.
fn main() {
    let s = "abc";
    let b: &mut [u8] = s.as_bytes();
}
// error: expected `&mut [u8]`, found `&[u8]`
