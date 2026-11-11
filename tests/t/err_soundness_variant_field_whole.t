// A variant's field refined as a whole: a write to a field of it must keep
// that.
#[derive(Copy, Discard)]
struct S { a: u8, b: u8 }

#[derive(Copy, Discard)]
enum E { A({ s: S | s.a < s.b }) }

fn main() {
    let mut e = E::A(S { a: 1, b: 2 });
    match &mut e {
        E::A(s) => s.a = 100,
    }
}
// error: can't prove `s.a < s.b`
