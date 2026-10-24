// Nor be freed.
use alloc::string::String;

fn main() with alloc {
    let s = String::from("a");
    let r = s.as_str();
    s.free();
    print_str(r);
}
// error: cannot move out of `s` while `s` is borrowed
