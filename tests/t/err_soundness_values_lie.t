// A function that says it leaves `old(*b)` behind `a` must: here it
// leaves a pair of its own (docs/85-values-in-predicates.md).
#[derive(Copy, Discard)]
struct Pair {
    lo: u8,
    hi: u8,
}

fn take(a: &mut Pair, b: &mut Pair) ensures *a == old(*b) { *a = Pair { lo: 0, hi: 0 }; }

fn main() {
    let mut p = Pair { lo: 1, hi: 2 };
    let mut q = Pair { lo: 3, hi: 4 };
    take(&mut p, &mut q);
}
// error: can't prove `*a == old(*b)` for what `take` ensures
