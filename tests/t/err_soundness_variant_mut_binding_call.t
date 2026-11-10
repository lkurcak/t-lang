// And through a `&mut` a call gives: what its type says of what it points
// to is said of it, which a write through `a` could break.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn get<'a>(t: &'a mut (Option<{ s: u8 | s < 64 }>, u8)) -> &'a mut Option<{ s: u8 | s < 64 }> {
    &mut t.0
}

fn main() {
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (square(1), 2);
    match get(&mut t) {
        Some(a) => *a = 99,
        _ => {},
    }
    let _ = t;
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
