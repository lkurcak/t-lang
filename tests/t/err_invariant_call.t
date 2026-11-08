// An invariant is a predicate, in the language of refinements: it calls
// only measures.

fn limit() -> usize { 10 }

fn f(n: usize) {
    let mut i: usize = 0;
    while i < n invariant i <= limit() {
        i = i + 1;
    }
}

fn main() {}
// error: an invariant can only use literals, names, measures, `+`, `-`, `*` by a constant, comparisons, `&&`, `||` and `!`
