// Where types must be equal, so must their effects, though one is only
// known once its group's are.
fn g(n: u8) -> i64 {
    let _keep: Option<fn(u8) -> i64> = Option::Some(g);
    if n == 9 { panic("boom") } else if n == 0 { 1 } else { g(n - 1) }
}

fn main() with panic { let _ = g(0); }
// error: expected the effects `{}`, found `panic`
