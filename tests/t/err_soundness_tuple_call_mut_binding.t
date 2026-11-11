// A tuple's field bound by `&mut` through a `&mut` a call gave: what the
// call's type says of the tuple, `*a = Some(99)` must keep.
fn square(n: u8) -> Option<{ s: u8 | s < 64 }> { if n < 64 { Some(n) } else { None } }

fn get<'a>(t: &'a mut (Option<{ s: u8 | s < 64 }>, u8)) -> &'a mut (Option<{ s: u8 | s < 64 }>, u8) {
    t
}

fn main() {
    let mut t: (Option<{ s: u8 | s < 64 }>, u8) = (square(1), 2);
    match get(&mut t) {
        (a, _) => *a = Some(99),
    }
    let _ = t;
}
// error: can't prove `s < 64` for what's written through `*a`
