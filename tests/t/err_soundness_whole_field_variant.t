// A struct in a variant's field, bound by `&mut`: a write to one of its
// fields must keep what the option's type says of the whole struct.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

fn poke(o: &mut Option<{ s: S | s.a < s.b }>) {
    match o {
        Some(s) => s.a = 9,
        None => {},
    }
}

fn main() {
    let mut o: Option<{ s: S | s.a < s.b }> = Some(S { a: 1, b: 2 });
    poke(&mut o);
}
// error: can't prove `s.a < s.b` for `s`, once `s.a` is written
