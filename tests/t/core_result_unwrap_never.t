// A `Result` whose error type is `!` is an `Ok`: `unwrap` gives what's in
// it, and can't panic there, so `main` says no `panic`
// (docs/90-conditional-effects.md).
use alloc::string::String;
use core::fmt::Write;

fn never_fails(x: i64) -> Result<i64, !> { Ok(x * 0 + 7) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&never_fails(0).unwrap())?; // expect: 7
    let mut s = String::new();
    let n = 3u8;
    match n {
        0 => s.write_str("none").unwrap(),
        _ => s.put(&n).unwrap(),
    }
    let w = out.line(&s.as_str()); // expect: 3
    s.discard();
    w
}
