// What an effect parameter is decided as, at a call, counts too.
fn g() -> i64 { panic("boom") }

fn apply<effect e, F: Fn() -> i64 with e + Discard>(f: F) -> i64 with e { f() }

export fn f() -> i64 { apply(g) }

fn main() with panic { let _ = f(); }
// error: `apply` may panic, but `f` doesn't say `with panic`
