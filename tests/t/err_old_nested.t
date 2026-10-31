// `old(old(x))` would be the same as `old(x)`, so it's an error, likely a
// typo.
#[derive(Copy, Discard)]
struct Counter { n: usize }

impl Counter {
    #[measure]
    fn get(&self) -> usize { self.n }
}

fn f(c: &mut Counter) ensures old(old(c.get())) == 0 {}

fn main() {}
// error: `old(..)` inside `old(..)`
