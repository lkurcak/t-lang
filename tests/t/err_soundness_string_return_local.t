// A `&str` from `as_str` can't outlive the borrow of the string it's
// from.
use alloc::string::String;

fn view(s: &String) -> &'static str { s.as_str() }

fn main() with {alloc, panic} {
    let s = String::from("a");
    let r = view(&s);
    s.free();
    let _v = r;
}
// error: lifetime may not live long enough: this needs the lifetime of `s` to outlive `'static`, and the signature doesn't say it does
