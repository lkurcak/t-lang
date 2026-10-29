// The error at the exported function says why what it calls has the
// effect, call by call, as it's nowhere written.
fn h() -> i64 { panic("boom") }

fn g() -> i64 { h() }

export fn f() -> i64 { g() }

fn main() with panic { let _ = f(); }
// error: `h` may panic here: `panic` may
