// An inferred function's type has its effects, so it doesn't fit a
// function pointer that allows fewer.
fn g() -> i64 { panic("boom") }

fn main() with panic {
    let p: fn() -> i64 = g;
    let _ = p();
}
// error: expected `fn() -> i64`, found `fn() -> i64 with panic`
