// An `if` in a measure's body gives one of two terms of the logic: a
// branch with a `let` in it isn't one (docs/88-iterator-laws.md).
#[derive(Discard)]
struct S { a: usize }

impl S {
    #[measure]
    fn m(&self) -> usize {
        if self.a > 0 {
            let b = self.a;
            b
        } else {
            0
        }
    }
}

fn main() {}
// error: a measure's body can only use `self`'s fields, measures, literals, `+`, `-`, `*`, `/` and `%` by a constant, comparisons, `&&`, `||`, `!`, and `if` with an `else`
