// An or-pattern's `&mut` binding is of either variant's field: what's
// written must have both's refinements.
#[derive(Copy, Discard)]
enum E { A({ x: u8 | x < 3 }), B(u8) }

fn main() {
    let mut e = E::B(9);
    match &mut e {
        E::A(x) | E::B(x) => *x = 200,
    }
}
// error: can't prove `x < 3`
