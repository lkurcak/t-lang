// A string's field is private, so nothing outside `alloc::string` can
// make one of bytes that aren't UTF-8.
use alloc::list::List;
use alloc::string::String;

fn main() with {alloc, panic} {
    let mut l: List<u8> = List::new();
    l.push(0xFF);
    let s = String { bytes: l };
    let _v = s.as_str();
    s.free();
}
// error: field `bytes` of `String` is private to `alloc::string`
