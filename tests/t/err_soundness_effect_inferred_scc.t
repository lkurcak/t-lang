// In a recursive group, an effect of one is every one's that can reach it.
fn a(n: u8) -> u8 { if n == 0 { 0 } else { b(n - 1) } }

fn b(n: u8) -> u8 { if n == 7 { panic("boom") } else { a(n) } }

export fn f() -> u8 { a(9) }

fn main() with panic { let _ = f(); }
// error: `a` may panic, but `f` doesn't say `with panic`
