// `into_bytes` uses the string up.
use alloc::string::String;

fn main() with {alloc, panic} {
    let s = String::from("a");
    let l = s.into_bytes();
    print_str(s.as_str());
    l.free();
}
// error: use of moved value `s`
