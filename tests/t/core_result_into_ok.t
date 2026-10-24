// A `Result` whose error type is `!` is an `Ok`: `into_ok` gives what's in
// it, and can't panic, so `main` says no `panic`.
use alloc::string::String;
use core::fmt::Write;

fn never_fails(x: i64) -> Result<i64, !> { Result::Ok(x * 0 + 7) }

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    out.line(&never_fails(0).into_ok())?; // expect: 7
    let mut s = String::new();
    let n = 3u8;
    match n {
        0 => s.write_str("none").into_ok(),
        _ => s.put(&n).into_ok(),
    }
    let w = out.line(&s.as_str()); // expect: 3
    s.free();
    w
}
