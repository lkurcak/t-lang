// From inside its own group too, once its effects are known.
fn g(n: u8) -> i64 {
    let p: fn(u8) -> i64 = g;
    if n == 0 { panic("boom") } else { p(n - 1) }
}

fn main() with panic { let _ = g(1); }
// error: expected `fn(u8) -> i64`, found `fn(u8) -> i64 with panic`
