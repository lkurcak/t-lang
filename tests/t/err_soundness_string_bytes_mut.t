// `as_bytes` only lends the bytes to be read: writing one could make them
// not UTF-8.
use alloc::string::String;

fn main() with alloc {
    let s = String::from("a");
    match s.as_bytes().get_mut(0) {
        Option::Some(b) => *b = 0xFF,
        Option::None => {},
    }
    print_str(s.as_str());
    s.free();
}
// error: cannot borrow `*a temporary` as mutable: it's behind a `&` reference, `a temporary`, and only what's behind a `&mut` can be changed
