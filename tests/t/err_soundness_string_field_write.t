// Nor can it change a string's bytes behind its back.
use alloc::string::String;

fn main() with {alloc, panic} {
    let mut s = String::from("a");
    s.bytes.push(0xFF);
    print_str(s.as_str());
    s.free();
}
// error: field `bytes` of `String` is private to `alloc::string`
