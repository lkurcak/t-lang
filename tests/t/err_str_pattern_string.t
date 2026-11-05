// A `String` is not a `&str`: match on `s.as_str()`.
fn f(s: alloc::string::String) -> u8 {
    match s {
        "1" => 1,
        _ => 0,
    }
}
fn main() {}
// error: expected `String`, found `&'static str`
