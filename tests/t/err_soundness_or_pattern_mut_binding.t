// What an alternative of an or-pattern binds by `&mut` is a `&mut` of
// whichever variant's field matched: what's written through it must keep
// what's declared of both. (It wrote 99 into an `E<{ s: u8 | s < 64 }>`
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
// error: can't prove `s < 64` for what's written through `*a`
