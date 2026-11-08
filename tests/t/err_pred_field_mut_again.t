// A `mut` variable's refinement is checked at each assignment, after which
// what's behind a `&mut` may have changed: so it can't name it.
#[derive(Discard)]
struct C { n: u8 }

fn f(c: &mut C) {
    let mut i: { i: u8 | i <= c.n } = 0;
    i = 0;
    let _ = i;
}

fn main() {}
// error: the refinement of a `mut` variable can't name a field of what's behind `&mut` `c`, which can change
