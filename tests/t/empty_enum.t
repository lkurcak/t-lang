// An enum with no variants has no values at all: it's the empty type, 0 in
// the algebra of types. A `match` on it needs no arms, and can have any type,
// because it can never run.

enum Never {}

fn absurd(n: Never) -> int {
    match n {}
}

// This function can't be called, since no `Never` exists to pass it. Its
// type says "if you give me the impossible, I'll give you anything".
fn absurd_bool(n: Never) -> bool { match n {} }

// No value can be built with `Maybe::Impossible`, since it needs a `Never`.
// So it needs no arm: matching `Real` covers every `Maybe` there is.
enum Maybe { Real(int), Impossible(Never) }

fn get(m: Maybe) -> int {
    match m {
        Maybe::Real(x) => x,
    }
}

fn main() {
    print(get(Maybe::Real(1)));   // expect: 1
}
// Neither function can ever be called, and `Impossible` never constructed.
// warning: function `absurd` is never used
// warning: function `absurd_bool` is never used
// warning: variant `Maybe::Impossible` is never constructed
