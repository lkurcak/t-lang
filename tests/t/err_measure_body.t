// A measure's body is a term of the logic: fields, measures, literals,
// arithmetic, comparisons and `if`. A `let` isn't one.
#[derive(Copy, Discard)]
struct W { n: usize }

impl W {
    #[measure]
    fn len(&self) -> usize {
        let k = self.n;
        k
    }
}

fn main() {}
// error: a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*`, `/` and `%` by a constant, comparisons, `&&`, `||`, `!`, and `if` with an `else`
