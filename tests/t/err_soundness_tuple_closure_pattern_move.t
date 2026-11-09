// A closure's pattern binds by reference through a reference: it can't
// move what it borrows.
use alloc::string::String;

fn keep(_s: String) {}

fn main() with alloc {
    let t = (String::from("a"), 1);
    let f = |(s, _): &(String, i64)| keep(s);
    f(&t);
}
// error: expected `String`, found `&String`
