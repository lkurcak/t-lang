// The same through a field of a tuple a `let` declares refined: what's
// declared of `t.0` is `t`'s, which a write through `a` must keep.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn main() {
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (square(1), 2);
    match &mut t.0 {
        Some(a) => *a = 99,
        _ => {},
    }
    let _ = t;
}
// error: can't prove `s < 64` for what's written through `*a`
