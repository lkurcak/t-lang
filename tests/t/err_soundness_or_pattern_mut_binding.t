// What an alternative of an or-pattern binds by `&mut` has no place the
// checker follows, as a variant's field hasn't: so it's refused where the
// value it's in is refined. (It wrote 99 into an `E<{ s: u8 | s < 64 }>`
// before step 62.)
#[derive(Copy, Discard)]
enum E<T> { A(T), B(T) }

fn poke(o: &mut E<{ s: u8 | s < 64 }>) {
    match o {
        E::A(a) | E::B(a) => *a = 99,
    }
}

fn main() {
    let mut o: E<{ s: u8 | s < 64 }> = E::B(3);
    poke(&mut o);
}
// error: can't bind `a` by `&mut`: what it's in has a refinement, which a write through the reference could break
