// A `&mut` to a struct's field refined alone says so: what's written
// through it must have the refinement.
#[derive(Copy, Discard)]
struct S { a: { v: u8 | v < 3 }, b: u8 }

fn main() {
    let mut s = S { a: 1, b: 2 };
    let r = &mut s.a;
    *r = 3;
}
// error: can't prove `v < 3`
