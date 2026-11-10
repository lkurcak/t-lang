// The same through a field of a tuple a `let` declares refined: `t.0`'s
// refinement is `t`'s, which a write through `a` could break.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn main() {
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (square(1), 2);
    match &mut t.0 {
        Some(a) => *a = 99,
        _ => {},
    }
    let _ = t;
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
