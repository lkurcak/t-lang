// Inside generic code too: a call of a trait's method is known to be of
// the impl `select` finds for its `Self` type when that type has type
// parameters in it, as `Iter<'_, (usize, E)>` does in `impl<E>`: any type
// they stand for, it's that impl, as another would overlap it
// (docs/60-refined-type-arguments.md). So `for` gives what the list holds.
use alloc::list::List;

#[derive(Discard where E: Discard)]
struct Edges<E> { out: List<({ t: usize | t < 4 }, E)> }

impl<E: Discard> Edges<E> {
    fn new() -> Edges<E> { Edges { out: List::new() } }

    fn add(&mut self, to: { t: usize | t < 4 }, e: E) with alloc { self.out.push((to, e)) }

    fn targets(&self, a: [u64; 4]) -> u64 {
        let mut s: u64 = 0;
        for (t, _) in self.out.iter() {
            s = wrapping_add(s, a[*t]);
        }
        s
    }
}

// Generic in the element too: whatever `T` is, the list's elements are
// what was pushed.
fn last_or<T: Copy>(l: &List<T>, d: T) -> T {
    match l.last() {
        Some(x) => *x,
        None => d,
    }
}

fn main(os: std::Os) -> Result<(), std::io::Error> with alloc {
    let mut out = os.stdout;
    let a: [u64; 4] = [1, 10, 100, 1000];
    let mut e: Edges<bool> = Edges::new();
    e.add(1, true);
    e.add(3, false);
    out.line(&e.targets(a))?; // expect: 1010
    let mut l: List<{ t: usize | t < 4 }> = List::new();
    l.push(2);
    out.line(&a[last_or(&l, 0)])?; // expect: 100
    Ok(())
}
