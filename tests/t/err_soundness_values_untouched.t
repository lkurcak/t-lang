// Two values nothing says are the same aren't: a pair `*a` is left as it
// was isn't the pair `*b` points to (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
struct Pair {
    lo: u8,
    hi: u8,
}

fn same(a: &mut Pair, b: &Pair) ensures *a == *b {}

fn main() {
    let mut p = Pair { lo: 1, hi: 2 };
    let q = Pair { lo: 3, hi: 4 };
    same(&mut p, &q);
}
// error: can't prove `*a == *b` for what `same` ensures
