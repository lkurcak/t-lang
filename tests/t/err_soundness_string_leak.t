// A string isn't `Discard`: dropping it would leak its buffer.
use alloc::string::String;

fn main() with alloc {
    let mut s = String::from("a");
    s.push('b');
}
// error: `s` is never used up, and its type, `String`, isn't `Discard`
