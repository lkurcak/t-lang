// A literal's bytes are the program's, shared by every run of the code
// that names it: they can't be lent mutably.
fn main() {
    let b: &mut [u8] = b"abc";
}
// error: expected `&mut [u8]`, found `&'static [u8]`
