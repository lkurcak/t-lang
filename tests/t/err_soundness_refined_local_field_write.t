// A refined `let mut`'s refinement is an invariant, of its whole value: a
// write to a field must keep it, as an assignment of the whole must. (Was
// unchecked before 0.74.0: `get(p)` then trusted `p.a < 3` of `p.a == 7`.)
#[derive(Copy, Discard)]
struct P { a: usize, b: usize }

fn get(p: { p: P | p.a < 3 }) -> usize { p.a }

fn main() {
    let mut p: { p: P | p.a < 3 } = P { a: 1, b: 2 };
    p.a = 7;
    let _ = get(p);
}
// error: can't prove `p.a < 3` for `p`
