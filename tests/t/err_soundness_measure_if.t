// An `if` in a measure's body is each branch only where its condition
// says: the least of two isn't the first when the second is less
// (docs/88-iterator-laws.md).
#[derive(Discard)]
struct Two { a: usize, b: usize }

impl Two {
    #[measure]
    fn least(&self) -> usize {
        if self.a <= self.b { self.a } else { self.b }
    }
}

fn first(t: &Two) -> { n: usize | n == t.a } { t.least() }

fn main() {
    let _ = first(&Two { a: 5, b: 1 });
}
// error: can't prove `n == t.a` for the result of `first`
