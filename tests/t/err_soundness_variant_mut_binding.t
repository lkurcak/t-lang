// A variant's field bound by `&mut` through match ergonomics is a `&mut`
// of that place: what's written through it must keep what the option's
// type says of it. (It wrote 99 into an `Option<{ s: u8 | s < 64 }>`
// before step 62, which refused the binding; step 66 checks the write.)
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
// error: can't prove `s < 64` for what's written through `*a`
