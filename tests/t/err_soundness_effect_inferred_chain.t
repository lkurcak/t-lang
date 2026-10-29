// An exported function says its effects, and what it calls must have no
// more, inferred or not: `g` can panic, through `h`.
fn h() -> i64 { panic("boom") }

fn g() -> i64 { h() }

export fn f() -> i64 { g() }

fn main() with panic { let _ = f(); }
// error: `g` may panic, but `f` doesn't say `with panic`
