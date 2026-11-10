// A variant's field bound by `&mut` through match ergonomics: what's
// written through it must keep what the option's type says of it, which
// nothing would check, so the binding is refused. (It wrote 99 into an
// `Option<{ s: u8 | s < 64 }>` before step 62.)
fn poke(o: &mut Option<{ s: u8 | s < 64 }>) {
    match o {
        Some(a) => *a = 99,
        None => {},
    }
}

fn main() {
    let mut o: Option<{ s: u8 | s < 64 }> = None;
    poke(&mut o);
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
