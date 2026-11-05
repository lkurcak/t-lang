// A path to a trait's method takes its receiver as its first argument,
// and `Self` can be `str` or a slice there, as it can for a method call:
// `Debug::fmt(s, w)` is `s.fmt(w)`.
use alloc::string::String;
use core::fmt::{Debug, Display};

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let mut buf = String::new();
    let s: &str = "a\"b";
    Debug::fmt(s, &mut buf).into_ok();
    Display::fmt(s, &mut buf).into_ok();
    let a = [1u8, 2];
    let xs: &[u8] = a.as_slice();
    Debug::fmt(xs, &mut buf).into_ok();
    out.line(&buf)?; // expect: "a\"b"a"b[1, 2]
    Result::Ok(())
}
