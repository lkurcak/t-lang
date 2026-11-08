// A refinement naming a field is about its value then: after an
// assignment, the field is a new value it says nothing of.
#[derive(Discard)]
struct C { n: u8 }

fn f(c: &mut C, i: { i: u8 | i < c.n }) -> u8 {
    c.n = 0;
    let j: { j: u8 | j < c.n } = i;
    j
}

fn main() {}
// error: can't prove `j < c.n` for `j`
