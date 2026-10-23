// A `&str` from `as_str` borrows the string, so it can't grow, which may
// move its bytes, while the `&str` lives.
use alloc::string::String;

fn main() with {alloc, panic} {
    let mut s = String::from("a");
    let r = s.as_str();
    s.push_str("bcdefgh");
    print_str(r);
    s.free();
}
// error: cannot borrow `s` as mutable while `s` is borrowed
