// A tuple's field bound by `&mut` through match ergonomics has a place,
// `t.0`, so what's written through it is checked against what `t`
// declares there, as through `let r = &mut t.0;`.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn main() {
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (square(1), 2);
    match &mut t {
        (a, _) => *a = Some(99),
    }
    let _ = t;
}
// error: can't prove `s < 64`
